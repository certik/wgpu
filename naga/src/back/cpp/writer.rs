use super::{conv, BackendResult, Error, Options};
use crate::{
    proc, valid, Handle, Module, ShaderStage, TypeInner,
};
use alloc::{
    format,
    string::{String, ToString},
};
use core::fmt::Write;

pub struct Writer<'a, W> {
    out: &'a mut W,
    names: crate::FastHashMap<proc::NameKey, String>,
    namer: proc::Namer,
    module: Option<&'a Module>,
    info: Option<&'a valid::ModuleInfo>,
    current_function: Option<&'a crate::Function>,
}

impl<'a, W: Write> Writer<'a, W> {
    pub fn new(out: &'a mut W, _options: &Options) -> Self {
        Self {
            out,
            names: crate::FastHashMap::default(),
            namer: proc::Namer::default(),
            module: None,
            info: None,
            current_function: None,
        }
    }

    pub fn write(&mut self, module: &'a Module, info: &'a valid::ModuleInfo) -> BackendResult {
        self.module = Some(module);
        self.info = Some(info);

        // Generate C++ runtime header include
        writeln!(self.out, "#include \"wgsl_runtime.hpp\"")?;
        writeln!(self.out)?;

        // Write type definitions
        for (handle, ty) in module.types.iter() {
            self.write_type_definition(handle, ty)?;
        }

        // Write global variables
        for (handle, var) in module.global_variables.iter() {
            self.write_global_variable(handle, var)?;
        }

        // Write functions
        for (handle, func) in module.functions.iter() {
            self.write_function(handle, func)?;
        }

        // Write entry points
        for (index, ep) in module.entry_points.iter().enumerate() {
            self.write_entry_point(ep, index)?;
        }

        Ok(())
    }

    fn write_type_definition(
        &mut self,
        _handle: Handle<crate::Type>,
        ty: &crate::Type,
    ) -> BackendResult {
        let module = self.module.unwrap();

        if let Some(name) = ty.name.as_ref() {
            match ty.inner {
                TypeInner::Struct { ref members, .. } => {
                    writeln!(self.out, "struct {} {{", name)?;
                    for member in members {
                        let member_name = member.name.as_ref().map(|s| s.as_str()).unwrap_or("_field");
                        write!(self.out, "    ")?;
                        self.write_type(&member.ty, &module.types)?;
                        writeln!(self.out, " {};", member_name)?;
                    }
                    writeln!(self.out, "}};")?;
                    writeln!(self.out)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn write_type(
        &mut self,
        ty_handle: &Handle<crate::Type>,
        types: &crate::UniqueArena<crate::Type>,
    ) -> BackendResult {
        let ty = &types[*ty_handle];

        match ty.inner {
            TypeInner::Scalar(scalar) => {
                write!(self.out, "{}", conv::scalar_to_cpp_type(scalar))?;
            }
            TypeInner::Vector { size, scalar } => {
                let vec_size = conv::vector_size_to_usize(size);
                write!(
                    self.out,
                    "vec{}<{}>",
                    vec_size,
                    conv::scalar_to_cpp_type(scalar)
                )?;
            }
            TypeInner::Matrix { columns, rows, scalar } => {
                let col_size = conv::vector_size_to_usize(columns);
                let row_size = conv::vector_size_to_usize(rows);
                write!(
                    self.out,
                    "mat{}x{}<{}>",
                    col_size,
                    row_size,
                    conv::scalar_to_cpp_type(scalar)
                )?;
            }
            TypeInner::Array { base, size, .. } => {
                write!(self.out, "std::array<")?;
                self.write_type(&base, types)?;
                write!(self.out, ", ")?;
                match size {
                    crate::ArraySize::Constant(_) => {
                        // For now, just use a placeholder
                        write!(self.out, "1")?;
                    }
                    crate::ArraySize::Dynamic => {
                        return Err(Error::Unsupported("dynamic arrays".to_string()));
                    }
                    crate::ArraySize::Pending(_) => {
                        return Err(Error::Unsupported("pending array size".to_string()));
                    }
                }
                write!(self.out, ">")?;
            }
            TypeInner::Struct { .. } => {
                if let Some(name) = ty.name.as_ref() {
                    write!(self.out, "{}", name)?;
                } else {
                    return Err(Error::Unsupported("anonymous structs".to_string()));
                }
            }
            TypeInner::Pointer { base, .. } => {
                self.write_type(&base, types)?;
                write!(self.out, "*")?;
            }
            _ => {
                return Err(Error::Unsupported(format!("type {:?}", ty.inner)));
            }
        }
        Ok(())
    }

    fn write_global_variable(
        &mut self,
        handle: Handle<crate::GlobalVariable>,
        var: &crate::GlobalVariable,
    ) -> BackendResult {
        let _ = handle; // Suppress unused warning
        let _module = self.module.unwrap();

        // Only write storage buffers for now
        match var.space {
            crate::AddressSpace::Storage { .. } | crate::AddressSpace::Uniform => {
                if let Some(name) = var.name.as_ref() {
                    let module = self.module.unwrap();
                    self.write_type(&var.ty, &module.types)?;
                    writeln!(self.out, " {};", name)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn write_function(
        &mut self,
        handle: Handle<crate::Function>,
        func: &'a crate::Function,
    ) -> BackendResult {
        let module = self.module.unwrap();

        // Get function name
        let func_name = func.name.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("unnamed_function");

        // Write return type
        if let Some(ref result) = func.result {
            self.write_type(&result.ty, &module.types)?;
        } else {
            write!(self.out, "void")?;
        }

        write!(self.out, " {}(", func_name)?;

        // Write parameters
        for (index, arg) in func.arguments.iter().enumerate() {
            if index > 0 {
                write!(self.out, ", ")?;
            }
            self.write_type(&arg.ty, &module.types)?;
            write!(self.out, " ")?;
            if let Some(ref name) = arg.name {
                write!(self.out, "{}", name)?;
            } else {
                write!(self.out, "param{}", index)?;
            }
        }

        writeln!(self.out, ") {{")?;

        // Set current function context for expression handling
        self.current_function = Some(func);

        // Write function body
        self.write_block(&func.body, 1)?;

        // Clear function context
        self.current_function = None;

        writeln!(self.out, "}}")?;
        writeln!(self.out)?;
        Ok(())
    }

    fn write_entry_point(
        &mut self,
        ep: &'a crate::EntryPoint,
        _index: usize,
    ) -> BackendResult {
        let _module = self.module.unwrap();

        // Only support compute shaders for now
        if ep.stage != ShaderStage::Compute {
            return Err(Error::InvalidStage(ep.stage));
        }

        // Write function signature
        writeln!(self.out, "void {}() {{", ep.name)?;

        // Set current function context
        self.current_function = Some(&ep.function);

        // Write function body
        self.write_block(&ep.function.body, 1)?;

        // Clear function context
        self.current_function = None;

        writeln!(self.out, "}}")?;
        writeln!(self.out)?;
        Ok(())
    }

    fn write_block(&mut self, block: &[crate::Statement], indent: usize) -> BackendResult {
        for statement in block {
            self.write_statement(statement, indent)?;
        }
        Ok(())
    }

    fn write_statement(&mut self, statement: &crate::Statement, indent: usize) -> BackendResult {
        use crate::Statement as St;

        let indent_str = "    ".repeat(indent);

        match *statement {
            St::Emit(_) => {}
            St::Block(ref block) => {
                self.write_block(block, indent)?;
            }
            St::If { condition, ref accept, ref reject } => {
                write!(self.out, "{}if (", indent_str)?;
                self.write_expr_handle(condition)?;
                writeln!(self.out, ") {{")?;
                self.write_block(accept, indent + 1)?;
                if !reject.is_empty() {
                    writeln!(self.out, "{}}} else {{", indent_str)?;
                    self.write_block(reject, indent + 1)?;
                }
                writeln!(self.out, "{}}}", indent_str)?;
            }
            St::Return { value } => {
                write!(self.out, "{}return", indent_str)?;
                if let Some(expr) = value {
                    write!(self.out, " ")?;
                    self.write_expr_handle(expr)?;
                }
                writeln!(self.out, ";")?;
            }
            St::Store { pointer, value } => {
                write!(self.out, "{}", indent_str)?;
                self.write_expr_handle(pointer)?;
                write!(self.out, " = ")?;
                self.write_expr_handle(value)?;
                writeln!(self.out, ";")?;
            }
            _ => {
                writeln!(self.out, "{}// Unsupported statement", indent_str)?;
            }
        }
        Ok(())
    }

    fn write_expr_handle(&mut self, handle: Handle<crate::Expression>) -> BackendResult {
        // Get expression from current function context
        let func = self.current_function.expect("No function context set");
        let expr = &func.expressions[handle];
        self.write_expression(expr, handle)
    }

    fn write_expression(
        &mut self,
        expr: &crate::Expression,
        _handle: Handle<crate::Expression>,
    ) -> BackendResult {
        use crate::Expression as Ex;

        match *expr {
            Ex::Literal(ref literal) => {
                self.write_literal(literal)?;
            }
            Ex::Constant(_) => {
                write!(self.out, "/* constant */")?;
            }
            Ex::GlobalVariable(handle) => {
                let module = self.module.unwrap();
                let var = &module.global_variables[handle];
                if let Some(name) = var.name.as_ref() {
                    write!(self.out, "{}", name)?;
                } else {
                    write!(self.out, "_global{}", handle.index())?;
                }
            }
            Ex::LocalVariable(_) => {
                write!(self.out, "/* local */")?;
            }
            Ex::FunctionArgument(index) => {
                let func = self.current_function.expect("No function context for FunctionArgument");
                if let Some(arg) = func.arguments.get(index as usize) {
                    if let Some(ref name) = arg.name {
                        write!(self.out, "{}", name)?;
                    } else {
                        write!(self.out, "param{}", index)?;
                    }
                } else {
                    write!(self.out, "/* invalid arg {} */", index)?;
                }
            }
            Ex::Binary { op, left, right } => {
                write!(self.out, "(")?;
                self.write_expr_handle(left)?;
                write!(self.out, " {} ", conv::binary_op_to_cpp(op))?;
                self.write_expr_handle(right)?;
                write!(self.out, ")")?;
            }
            Ex::Unary { op, expr } => {
                write!(self.out, "{}", conv::unary_op_to_cpp(op))?;
                self.write_expr_handle(expr)?;
            }
            Ex::AccessIndex { base, index } => {
                self.write_expr_handle(base)?;
                write!(self.out, "[{}]", index)?;
            }
            _ => {
                write!(self.out, "/* expr */")?;
            }
        }
        Ok(())
    }

    fn write_literal(&mut self, literal: &crate::Literal) -> BackendResult {
        match *literal {
            crate::Literal::F64(value) => write!(self.out, "{}", value)?,
            crate::Literal::F32(value) => write!(self.out, "{}f", value)?,
            crate::Literal::U32(value) => write!(self.out, "{}u", value)?,
            crate::Literal::I32(value) => write!(self.out, "{}", value)?,
            crate::Literal::I64(value) => write!(self.out, "{}L", value)?,
            crate::Literal::U64(value) => write!(self.out, "{}UL", value)?,
            crate::Literal::Bool(value) => write!(self.out, "{}", value)?,
            crate::Literal::F16(_) => {
                return Err(Error::Unsupported("f16 literals".to_string()));
            }
            crate::Literal::AbstractInt(_) | crate::Literal::AbstractFloat(_) => {
                write!(self.out, "/* abstract */")?;
            }
        }
        Ok(())
    }
}
