use alloc::{collections::BTreeMap as Map, vec::Vec};
use parity_wasm::{
	builder,
	elements::{self, FunctionType, Instruction, Instructions, Internal},
};

use super::{
	instrument_call,
	max_height::{MaxStackHeightCounter, MaxStackHeightCounterContext},
	resolve_func_type, Context,
};

struct Thunk {
	signature: FunctionType,
	body: Option<Vec<Instruction>>,
	// Index in function space of this thunk.
	idx: Option<u32>,
}

pub fn generate_thunks<I: IntoIterator<Item = Instruction>>(
	ctx: &mut Context,
	module: elements::Module,
	injection_fn: impl Fn(&FunctionType) -> I,
) -> Result<elements::Module, &'static str>
where
	I::IntoIter: ExactSizeIterator + Clone,
{
	// First, we need to collect all function indices that should be replaced by thunks
	let mut replacement_map: Map<u32, Thunk> = {
		// Replacement map is at least export section size.
		let mut replacement_map: Map<u32, Thunk> = Map::new();

		let mut maybe_context: Option<MaxStackHeightCounterContext> = None;

		for func_idx in thunk_function_indexes(&module) {
			let mut callee_stack_cost =
				ctx.stack_cost(func_idx).ok_or("function index isn't found")?;

			// Don't generate a thunk if stack_cost of a callee is zero.
			if callee_stack_cost != 0 {
				let signature = resolve_func_type(func_idx, &module)?.clone();
				let body_of_condition = injection_fn(&signature).into_iter();

				// Thunk body consist of:
				//  - preamble
				//  - argument pushing
				//  - original call
				//  - postamble
				//  - end

				// To pre-allocate memory, we need to count `8 + N + 6`, i.e. `14 + N`.
				// See `instrument_call` function for details.
				let mut thunk_body: Vec<Instruction> = Vec::with_capacity(
					signature.params().len() + (14 + body_of_condition.len()) + 1,
				);

				let arguments = signature
					.params()
					.iter()
					.enumerate()
					.map(|(arg_idx, _)| Instruction::GetLocal(arg_idx as u32));

				const CALLEE_STACK_COST_PLACEHOLDER: i32 = 1248163264;
				instrument_call(
					&mut thunk_body,
					func_idx,
					CALLEE_STACK_COST_PLACEHOLDER,
					ctx.stack_height_global_idx(),
					ctx.stack_limit(),
					body_of_condition,
					arguments,
				);

				thunk_body.push(Instruction::End);

				// Try to initialize MaxStackHeightCounterContext once
				if maybe_context.is_none() {
					maybe_context = Some((&module).try_into()?);
				}

				// Update callee_stack_cost to charge for the thunk call itself
				let context =
					maybe_context.expect("MaxStackHeightCounterContext must be initialized");
				let thunk_cost = MaxStackHeightCounter::new_with_context(context, &injection_fn)
					.compute_for_raw_func(&signature, &thunk_body)?;

				callee_stack_cost = callee_stack_cost
					.checked_add(thunk_cost)
					.ok_or("overflow during callee_stack_cost calculation")?;

				// Update thunk body with new cost
				for instruction in thunk_body
					.iter_mut()
					.filter(|i| **i == Instruction::I32Const(CALLEE_STACK_COST_PLACEHOLDER))
				{
					*instruction = Instruction::I32Const(callee_stack_cost as i32);
				}

				replacement_map
					.insert(func_idx, Thunk { signature, body: Some(thunk_body), idx: None });
			}
		}

		replacement_map
	};

	// Then, we generate a thunk for each original function.

	// Save current func_idx
	let mut next_func_idx = module.functions_space() as u32;

	let mut mbuilder = builder::from_module(module);
	for thunk in replacement_map.values_mut() {
		// TODO: Don't generate a signature, but find an existing one.

		let thunk_body = thunk.body.take().expect("can't get thunk function body");

		mbuilder = mbuilder
			.function()
			// Signature of the thunk should match the original function signature.
			.signature()
			.with_params(thunk.signature.params().to_vec())
			.with_results(thunk.signature.results().to_vec())
			.build()
			.body()
			.with_instructions(Instructions::new(thunk_body))
			.build()
			.build();

		thunk.idx = Some(next_func_idx);
		next_func_idx += 1;
	}
	let mut module = mbuilder.build();

	// And finally, fixup thunks in export and table sections.

	// Fixup original function index to a index of a thunk generated earlier.
	let fixup = |function_idx: &mut u32| {
		// Check whether this function is in replacement_map, since
		// we can skip thunk generation (e.g. if stack_cost of function is 0).
		if let Some(thunk) = replacement_map.get(function_idx) {
			*function_idx =
				thunk.idx.expect("At this point an index must be assigned to each thunk");
		}
	};

	for section in module.sections_mut() {
		match section {
			elements::Section::Export(export_section) =>
				for entry in export_section.entries_mut() {
					if let Internal::Function(function_idx) = entry.internal_mut() {
						fixup(function_idx)
					}
				},
			elements::Section::Element(elem_section) =>
				for segment in elem_section.entries_mut() {
					for function_idx in segment.members_mut() {
						fixup(function_idx)
					}
				},
			elements::Section::Start(start_idx) => fixup(start_idx),
			_ => {},
		}
	}

	Ok(module)
}

fn thunk_function_indexes(module: &elements::Module) -> impl Iterator<Item = u32> + '_ {
	let exports = module.export_section().map(|es| es.entries()).unwrap_or(&[]);
	let elem_segments = module.elements_section().map(|es| es.entries()).unwrap_or(&[]);
	let start_func_idx = module.start_section();

	let exported_func_indices = exports.iter().filter_map(|entry| match entry.internal() {
		Internal::Function(function_idx) => Some(*function_idx),
		_ => None,
	});
	let table_func_indices = elem_segments.iter().flat_map(|segment| segment.members()).cloned();

	exported_func_indices.chain(table_func_indices).chain(start_func_idx)
}
