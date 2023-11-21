//! Auxiliary functions for working with WASM modules.

use parity_wasm::elements::{IndexMap, Instruction, Internal, Module, Section};

/// Rewrites module sections after inserting imports.
pub fn rewrite_sections_after_insertion(
	mut module: Module,
	inserted_index: u32,
	inserted_count: u32,
) -> Result<Module, Module> {
	if inserted_count == 0 {
		return Err(module)
	}

	// Updating calling addresses (all calls to function index >= `inserted_index` should be
	// incremented)
	for section in module.sections_mut() {
		match section {
			Section::Code(code_section) =>
				for func_body in code_section.bodies_mut().iter_mut() {
					for instruction in func_body.code_mut().elements_mut().iter_mut() {
						if let Instruction::Call(call_index) = instruction {
							if *call_index >= inserted_index {
								*call_index += inserted_count
							}
						}
					}
				},
			Section::Export(export_section) =>
				for export in export_section.entries_mut() {
					if let Internal::Function(func_index) = export.internal_mut() {
						if *func_index >= inserted_index {
							*func_index += inserted_count
						}
					}
				},
			Section::Element(elements_section) => {
				// Note that we do not need to check the element type referenced because in the
				// WebAssembly 1.0 spec, the only allowed element type is funcref.
				for segment in elements_section.entries_mut() {
					// update all indirect call addresses initial values
					for func_index in segment.members_mut() {
						if *func_index >= inserted_index {
							*func_index += inserted_count
						}
					}
				}
			},
			Section::Start(start_idx) =>
				if *start_idx >= inserted_index {
					*start_idx += inserted_count
				},
			Section::Name(s) =>
				for functions in s.functions_mut() {
					*functions.names_mut() =
						IndexMap::from_iter(functions.names().iter().map(|(mut idx, name)| {
							if idx >= inserted_index {
								idx += inserted_count;
							}

							(idx, name.clone())
						}));
				},
			_ => {},
		}
	}

	Ok(module)
}
