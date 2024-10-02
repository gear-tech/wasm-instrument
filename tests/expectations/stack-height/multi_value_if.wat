(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (type (;2;) (func (result i32)))
  (func $f (;0;) (type 0) (param $param i32) (result i32)
    i32.const 1
    i32.const 2
    local.get $param
    if (type 1) (param i32 i32) (result i32) ;; label = @1
      i32.add
    else
      i32.sub
    end
  )
  (func $main (;1;) (type 2) (result i32)
    i32.const 0
    global.get 0
    i32.const 5
    i32.add
    global.set 0
    global.get 0
    i32.const 1024
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    call $f
    global.get 0
    i32.const 5
    i32.sub
    global.set 0
  )
  (func (;2;) (type 2) (result i32)
    global.get 0
    i32.const 10
    i32.add
    global.set 0
    global.get 0
    i32.const 1024
    i32.gt_u
    if ;; label = @1
      unreachable
    end
    call $main
    global.get 0
    i32.const 10
    i32.sub
    global.set 0
  )
  (global (;0;) (mut i32) i32.const 0)
  (export "main" (func 2))
)