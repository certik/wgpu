use super::{conv, keywords, BackendResult, Error, Options};
use crate::{
    proc::{self, NameKey},
    valid, Handle, Module, TypeInner,
};
use alloc::{
    format,
    string::{String, ToString},
};
use core::fmt::Write;

pub struct Writer<'a, W> {
    out: &'a mut W,
    names: crate::FastHashMap<NameKey, String>,
    namer: proc::Namer,
    module: Option<&'a Module>,
    info: Option<&'a valid::ModuleInfo>,
    current_function: Option<&'a crate::Function>,
    current_function_kind: Option<FunctionKind>,
    call_results: crate::FastHashMap<Handle<crate::Expression>, String>,
    temp_counter: usize,
}

#[derive(Clone, Copy)]
enum FunctionKind {
    Function(Handle<crate::Function>),
    EntryPoint(proc::EntryPointIndex),
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
            current_function_kind: None,
            call_results: crate::FastHashMap::default(),
            temp_counter: 0,
        }
    }

    pub fn write(&mut self, module: &'a Module, info: &'a valid::ModuleInfo) -> BackendResult {
        self.module = Some(module);
        self.info = Some(info);

        self.names.clear();
        self.namer.reset(
            module,
            &*keywords::RESERVED_SET,
            &*keywords::RESERVED_CASE_INSENSITIVE_SET,
            keywords::RESERVED_PREFIXES,
            &mut self.names,
        );

        // Generate C++ runtime header include
        writeln!(self.out, "#include \"wgsl_runtime.hpp\"")?;
        writeln!(self.out)?;

        // Write type definitions
        for (handle, ty) in module.types.iter() {
            self.write_type_definition(handle, ty)?;
        }

        // Write module constants
        for (handle, constant) in module.constants.iter() {
            self.write_constant(handle, constant)?;
        }

        if !module.constants.is_empty() {
            writeln!(self.out)?;
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

    fn name(&self, key: NameKey) -> &str {
        self.names
            .get(&key)
            .map(String::as_str)
            .expect("identifier missing from namer")
    }

    fn resolve_array_length(&self, size: &crate::ArraySize) -> Result<usize, Error> {
        match *size {
            crate::ArraySize::Constant(value) => Ok(value.get() as usize),
            crate::ArraySize::Dynamic => Err(Error::Unsupported("dynamic arrays".to_string())),
            crate::ArraySize::Pending(_) => {
                Err(Error::Unsupported("pending array size".to_string()))
            }
        }
    }

    fn write_constant(
        &mut self,
        handle: Handle<crate::Constant>,
        constant: &crate::Constant,
    ) -> BackendResult {
        let module = self.module.unwrap();
        let const_name = self.name(NameKey::Constant(handle)).to_string();

        write!(self.out, "constexpr ")?;
        self.write_type(&constant.ty, &module.types)?;
        write!(self.out, " {} = ", const_name)?;
        self.write_module_expression(constant.init)?;
        writeln!(self.out, ";")?;
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
                        let member_name =
                            member.name.as_ref().map(|s| s.as_str()).unwrap_or("_field");
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
            TypeInner::Matrix {
                columns,
                rows,
                scalar,
            } => {
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
                let length = self.resolve_array_length(&size)?;
                write!(self.out, "{}", length)?;
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
        let module = self.module.unwrap();

        match var.space {
            crate::AddressSpace::Storage { .. }
            | crate::AddressSpace::Uniform
            | crate::AddressSpace::Private
            | crate::AddressSpace::Function => {
                let var_name = self.name(NameKey::GlobalVariable(handle)).to_string();
                self.write_type(&var.ty, &module.types)?;
                write!(self.out, " {}", var_name)?;
                if let Some(init) = var.init {
                    write!(self.out, " = ")?;
                    self.write_module_expression(init)?;
                }
                writeln!(self.out, ";")?;
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
        let func_name = self.name(NameKey::Function(handle)).to_string();

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
            let key = NameKey::FunctionArgument(handle, index as u32);
            let arg_name = self.name(key).to_string();
            write!(self.out, "{}", arg_name)?;
        }

        writeln!(self.out, ") {{")?;

        // Set current function context for expression handling
        self.current_function = Some(func);
        self.current_function_kind = Some(FunctionKind::Function(handle));
        self.call_results.clear();
        self.temp_counter = 0;

        self.write_local_declarations(func, 1)?;

        // Write function body
        self.write_block(&func.body, 1)?;

        // Clear function context
        self.current_function = None;
        self.current_function_kind = None;

        writeln!(self.out, "}}")?;
        writeln!(self.out)?;
        Ok(())
    }

    fn write_local_declarations(
        &mut self,
        func: &'a crate::Function,
        indent: usize,
    ) -> BackendResult {
        if func.local_variables.is_empty() {
            return Ok(());
        }

        let indent_str = "    ".repeat(indent);
        let module = self.module.unwrap();
        let func_info = self.current_function_info();

        for (handle, local) in func.local_variables.iter() {
            let name_key = self.local_name_key(handle);
            let local_name = self.name(name_key).to_string();
            write!(self.out, "{}", indent_str)?;
            self.write_type(&local.ty, &module.types)?;
            write!(self.out, " {}", local_name)?;
            if let Some(init) = local.init {
                write!(self.out, " = ")?;
                self.write_expression_arena(init, &func.expressions, Some(func_info))?;
            }
            writeln!(self.out, ";")?;
        }

        writeln!(self.out)?;
        Ok(())
    }

    fn current_function_info(&self) -> &'a valid::FunctionInfo {
        let info = self.info.expect("module info not set");
        match self
            .current_function_kind
            .expect("no active function for current context")
        {
            FunctionKind::Function(handle) => &info[handle],
            FunctionKind::EntryPoint(index) => info.get_entry_point(index as usize),
        }
    }

    fn local_name_key(&self, local: Handle<crate::LocalVariable>) -> NameKey {
        match self
            .current_function_kind
            .expect("no active function for current context")
        {
            FunctionKind::Function(handle) => NameKey::FunctionLocal(handle, local),
            FunctionKind::EntryPoint(index) => NameKey::EntryPointLocal(index, local),
        }
    }

    fn argument_name(&self, index: usize) -> String {
        match self
            .current_function_kind
            .expect("no active function for current context")
        {
            FunctionKind::Function(handle) => self
                .name(NameKey::FunctionArgument(handle, index as u32))
                .to_string(),
            FunctionKind::EntryPoint(ep_index) => self
                .name(NameKey::EntryPointArgument(ep_index, index as u32))
                .to_string(),
        }
    }

    fn write_module_expression(&mut self, handle: Handle<crate::Expression>) -> BackendResult {
        let module = self.module.unwrap();
        self.write_expression_arena(handle, &module.global_expressions, None)
    }

    fn resolve_expression_type(
        &self,
        handle: Handle<crate::Expression>,
        func_info: Option<&'a valid::FunctionInfo>,
    ) -> &'a TypeInner {
        let module = self.module.unwrap();
        match func_info {
            Some(info) => info[handle].ty.inner_with(&module.types),
            None => self.info.unwrap()[handle].inner_with(&module.types),
        }
    }

    fn write_expression_arena(
        &mut self,
        handle: Handle<crate::Expression>,
        arena: &crate::Arena<crate::Expression>,
        func_info: Option<&valid::FunctionInfo>,
    ) -> BackendResult {
        let expr = &arena[handle];
        self.write_expression(expr, handle, arena, func_info)
    }

    fn ensure_call_result_name(
        &mut self,
        result: Handle<crate::Expression>,
        callee_handle: Handle<crate::Function>,
    ) -> String {
        if let Some(existing) = self.call_results.get(&result) {
            return existing.clone();
        }

        if let Some(func) = self.current_function {
            if let Some(name) = func.named_expressions.get(&result) {
                let owned = name.clone();
                self.call_results.insert(result, owned.clone());
                return owned;
            }
        }

        let base = self.name(NameKey::Function(callee_handle)).to_string();
        let generated = format!("{}_{}", base, self.temp_counter);
        self.temp_counter += 1;
        self.call_results.insert(result, generated.clone());
        generated
    }

    fn write_function_call(
        &mut self,
        function: Handle<crate::Function>,
        arguments: &[Handle<crate::Expression>],
    ) -> BackendResult {
        let func_name = self.name(NameKey::Function(function)).to_string();
        write!(self.out, "{}(", func_name)?;
        for (index, arg) in arguments.iter().enumerate() {
            if index > 0 {
                write!(self.out, ", ")?;
            }
            self.write_expr_handle(*arg)?;
        }
        write!(self.out, ")")?;
        Ok(())
    }

    fn write_entry_point(&mut self, ep: &'a crate::EntryPoint, index: usize) -> BackendResult {
        let module = self.module.unwrap();
        let ep_index = index as proc::EntryPointIndex;

        let ep_name = self.name(NameKey::EntryPoint(ep_index)).to_string();

        if let Some(ref result) = ep.function.result {
            self.write_type(&result.ty, &module.types)?;
        } else {
            write!(self.out, "void")?;
        }

        write!(self.out, " {}(", ep_name)?;

        for (arg_index, arg) in ep.function.arguments.iter().enumerate() {
            if arg_index > 0 {
                write!(self.out, ", ")?;
            }
            self.write_type(&arg.ty, &module.types)?;
            let arg_name = self
                .name(NameKey::EntryPointArgument(ep_index, arg_index as u32))
                .to_string();
            write!(self.out, " {}", arg_name)?;
        }

        writeln!(self.out, ") {{")?;

        self.current_function = Some(&ep.function);
        self.current_function_kind = Some(FunctionKind::EntryPoint(ep_index));
        self.call_results.clear();
        self.temp_counter = 0;

        self.write_local_declarations(&ep.function, 1)?;
        self.write_block(&ep.function.body, 1)?;

        self.current_function = None;
        self.current_function_kind = None;

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
        let inner_indent = "    ".repeat(indent + 1);

        match *statement {
            St::Emit(_) => {}
            St::Block(ref block) => {
                writeln!(self.out, "{}{{", indent_str)?;
                self.write_block(block, indent + 1)?;
                writeln!(self.out, "{}}}", indent_str)?;
            }
            St::If {
                condition,
                ref accept,
                ref reject,
            } => {
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
            St::Loop {
                ref body,
                ref continuing,
                break_if,
            } => {
                writeln!(self.out, "{}while (true) {{", indent_str)?;
                self.write_block(body, indent + 1)?;
                if !continuing.is_empty() {
                    self.write_block(continuing, indent + 1)?;
                }
                if let Some(expr) = break_if {
                    write!(self.out, "{}if (", inner_indent)?;
                    self.write_expr_handle(expr)?;
                    writeln!(self.out, ") {{")?;
                    writeln!(self.out, "{}    break;", inner_indent)?;
                    writeln!(self.out, "{}}}", inner_indent)?;
                }
                writeln!(self.out, "{}}}", indent_str)?;
            }
            St::Break => {
                writeln!(self.out, "{}break;", indent_str)?;
            }
            St::Continue => {
                writeln!(self.out, "{}continue;", indent_str)?;
            }
            St::Store { pointer, value } => {
                write!(self.out, "{}", indent_str)?;
                self.write_expr_handle(pointer)?;
                write!(self.out, " = ")?;
                self.write_expr_handle(value)?;
                writeln!(self.out, ";")?;
            }
            St::Call {
                function,
                ref arguments,
                result,
            } => {
                if let Some(result_handle) = result {
                    let name = self.ensure_call_result_name(result_handle, function);
                    let module = self.module.unwrap();
                    let callee = &module.functions[function];
                    let Some(ref callee_result) = callee.result else {
                        return Err(Error::Unsupported(
                            "call with result from void function".to_string(),
                        ));
                    };
                    write!(self.out, "{}", indent_str)?;
                    self.write_type(&callee_result.ty, &module.types)?;
                    write!(self.out, " {} = ", name)?;
                    self.write_function_call(function, arguments)?;
                    writeln!(self.out, ";")?;
                } else {
                    write!(self.out, "{}", indent_str)?;
                    self.write_function_call(function, arguments)?;
                    writeln!(self.out, ";")?;
                }
            }
            _ => {
                writeln!(self.out, "{}// Unsupported statement", indent_str)?;
            }
        }
        Ok(())
    }

    fn write_expr_handle(&mut self, handle: Handle<crate::Expression>) -> BackendResult {
        let func = self.current_function.expect("No function context set");
        let func_info = self.current_function_info();
        self.write_expression_arena(handle, &func.expressions, Some(func_info))
    }

    fn write_expression(
        &mut self,
        expr: &crate::Expression,
        handle: Handle<crate::Expression>,
        arena: &crate::Arena<crate::Expression>,
        func_info: Option<&valid::FunctionInfo>,
    ) -> BackendResult {
        use crate::Expression as Ex;

        match *expr {
            Ex::Literal(ref literal) => {
                self.write_literal(literal)?;
            }
            Ex::Constant(constant) => {
                let name = self.name(NameKey::Constant(constant)).to_string();
                write!(self.out, "{}", name)?;
            }
            Ex::GlobalVariable(global) => {
                let name = self.name(NameKey::GlobalVariable(global)).to_string();
                write!(self.out, "{}", name)?;
            }
            Ex::LocalVariable(local) => {
                let name = self.name(self.local_name_key(local)).to_string();
                write!(self.out, "{}", name)?;
            }
            Ex::FunctionArgument(index) => {
                let arg_name = self.argument_name(index as usize);
                write!(self.out, "{}", arg_name)?;
            }
            Ex::Binary { op, left, right } => {
                write!(self.out, "(")?;
                self.write_expression_arena(left, arena, func_info)?;
                write!(self.out, " {} ", conv::binary_op_to_cpp(op))?;
                self.write_expression_arena(right, arena, func_info)?;
                write!(self.out, ")")?;
            }
            Ex::Unary { op, expr } => {
                write!(self.out, "{}", conv::unary_op_to_cpp(op))?;
                self.write_expression_arena(expr, arena, func_info)?;
            }
            Ex::AccessIndex { base, index } => {
                self.write_expression_arena(base, arena, func_info)?;

                let module = self.module.unwrap();
                let mut ty = self.resolve_expression_type(base, func_info);

                loop {
                    match *ty {
                        TypeInner::Pointer { base: handle, .. } => {
                            ty = &module.types[handle].inner;
                        }
                        TypeInner::ValuePointer { .. } => {
                            break;
                        }
                        _ => break,
                    }
                }

                match *ty {
                    TypeInner::Struct { ref members, .. } => {
                        let member_name = members
                            .get(index as usize)
                            .and_then(|m| m.name.as_deref())
                            .unwrap_or("_field");
                        write!(self.out, ".{}", member_name)?;
                    }
                    _ => {
                        write!(self.out, "[{}]", index)?;
                    }
                }
            }
            Ex::Access { base, index } => {
                self.write_expression_arena(base, arena, func_info)?;
                write!(self.out, "[")?;
                self.write_expression_arena(index, arena, func_info)?;
                write!(self.out, "]")?;
            }
            Ex::Swizzle {
                size,
                vector,
                pattern,
            } => {
                use crate::VectorSize;
                let component_count = match size {
                    VectorSize::Bi => 2,
                    VectorSize::Tri => 3,
                    VectorSize::Quad => 4,
                };

                if component_count == 1 {
                    self.write_expression_arena(vector, arena, func_info)?;
                    write!(
                        self.out,
                        ".{}",
                        crate::back::COMPONENTS[pattern[0] as usize]
                    )?;
                } else {
                    let ty = self.resolve_expression_type(handle, func_info);
                    let scalar_type = match *ty {
                        TypeInner::Vector { scalar, .. } => scalar,
                        _ => {
                            return Err(Error::Unsupported(
                                "swizzle result is not a vector".to_string(),
                            ));
                        }
                    };
                    write!(
                        self.out,
                        "vec{}<{}>(",
                        component_count,
                        conv::scalar_to_cpp_type(scalar_type)
                    )?;
                    for i in 0..component_count {
                        if i > 0 {
                            write!(self.out, ", ")?;
                        }
                        self.write_expression_arena(vector, arena, func_info)?;
                        let component = pattern[i] as usize;
                        write!(self.out, ".{}", crate::back::COMPONENTS[component])?;
                    }
                    write!(self.out, ")")?;
                }
            }
            Ex::Derivative { axis, expr, .. } => {
                use crate::DerivativeAxis;
                match axis {
                    DerivativeAxis::Width => {
                        write!(self.out, "fwidth(")?;
                        self.write_expression_arena(expr, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    _ => {
                        return Err(Error::Unsupported(
                            "derivative axis not supported".to_string(),
                        ));
                    }
                }
            }
            Ex::As {
                expr,
                kind,
                convert,
            } => {
                let width = convert.unwrap_or(match kind {
                    crate::ScalarKind::Sint
                    | crate::ScalarKind::Uint
                    | crate::ScalarKind::Float => 4,
                    crate::ScalarKind::Bool => crate::BOOL_WIDTH,
                    crate::ScalarKind::AbstractInt | crate::ScalarKind::AbstractFloat => 4,
                });
                let scalar = crate::Scalar { kind, width };
                let cpp_type = conv::scalar_to_cpp_type(scalar);
                if convert.is_some() {
                    write!(self.out, "static_cast<{}>(", cpp_type)?;
                    self.write_expression_arena(expr, arena, func_info)?;
                    write!(self.out, ")")?;
                } else {
                    write!(self.out, "bitcast<{}>(", cpp_type)?;
                    self.write_expression_arena(expr, arena, func_info)?;
                    write!(self.out, ")")?;
                }
            }
            Ex::Load { pointer } => {
                self.write_expression_arena(pointer, arena, func_info)?;
            }
            Ex::Compose { ty, ref components } => {
                let module = self.module.unwrap();
                match module.types[ty].inner {
                    TypeInner::Vector { size, scalar } => {
                        let total_components = conv::vector_size_to_usize(size);
                        let mut written = 0usize;
                        write!(
                            self.out,
                            "vec{}<{}>(",
                            total_components,
                            conv::scalar_to_cpp_type(scalar)
                        )?;

                        let mut first = true;
                        for component in components {
                            let component_type =
                                self.resolve_expression_type(*component, func_info);
                            match *component_type {
                                TypeInner::Scalar(_) => {
                                    if !first {
                                        write!(self.out, ", ")?;
                                    }
                                    self.write_expression_arena(*component, arena, func_info)?;
                                    written += 1;
                                    first = false;
                                }
                                TypeInner::Vector {
                                    size: inner_size, ..
                                } => {
                                    let inner_components = conv::vector_size_to_usize(inner_size);
                                    for swizzle_index in 0..inner_components {
                                        if !first {
                                            write!(self.out, ", ")?;
                                        }
                                        self.write_expression_arena(*component, arena, func_info)?;
                                        write!(
                                            self.out,
                                            ".{}",
                                            crate::back::COMPONENTS[swizzle_index]
                                        )?;
                                        written += 1;
                                        first = false;
                                    }
                                }
                                _ => {
                                    return Err(Error::Unsupported(
                                        "unsupported vector constructor component".to_string(),
                                    ));
                                }
                            }
                        }

                        if written != total_components {
                            return Err(Error::Unsupported(
                                "vector constructor component count mismatch".to_string(),
                            ));
                        }

                        write!(self.out, ")")?;
                    }
                    TypeInner::Scalar(_) => {
                        if let Some(component) = components.first() {
                            self.write_expression_arena(*component, arena, func_info)?;
                        } else {
                            write!(self.out, "0")?;
                        }
                    }
                    TypeInner::Array { base, ref size, .. } => {
                        let length = self.resolve_array_length(size)?;
                        write!(self.out, "std::array<")?;
                        self.write_type(&base, &module.types)?;
                        write!(self.out, ", {}>{{", length)?;
                        for (index, component) in components.iter().enumerate() {
                            if index > 0 {
                                write!(self.out, ", ")?;
                            }
                            self.write_expression_arena(*component, arena, func_info)?;
                        }
                        write!(self.out, "}}")?;
                    }
                    _ => {
                        return Err(Error::Unsupported(
                            "composite constructors for this type".to_string(),
                        ));
                    }
                }
            }
            Ex::Splat { size, value } => {
                if let TypeInner::Vector { scalar, .. } =
                    *self.resolve_expression_type(handle, func_info)
                {
                    write!(
                        self.out,
                        "vec{}<{}>(",
                        conv::vector_size_to_usize(size),
                        conv::scalar_to_cpp_type(scalar)
                    )?;
                    self.write_expression_arena(value, arena, func_info)?;
                    write!(self.out, ")")?;
                } else {
                    return Err(Error::Unsupported("splat for non-vector type".to_string()));
                }
            }
            Ex::Select {
                condition,
                accept,
                reject,
            } => {
                write!(self.out, "select(")?;
                self.write_expression_arena(reject, arena, func_info)?;
                write!(self.out, ", ")?;
                self.write_expression_arena(accept, arena, func_info)?;
                write!(self.out, ", ")?;
                self.write_expression_arena(condition, arena, func_info)?;
                write!(self.out, ")")?;
            }
            Ex::Math {
                fun,
                arg,
                arg1,
                arg2,
                ..
            } => {
                use crate::MathFunction as Mf;
                match fun {
                    Mf::Abs => {
                        let ty = self.resolve_expression_type(handle, func_info);
                        match *ty {
                            TypeInner::Scalar(_) => {
                                write!(self.out, "std::abs(")?;
                                self.write_expression_arena(arg, arena, func_info)?;
                                write!(self.out, ")")?;
                            }
                            TypeInner::Vector { .. } => {
                                write!(self.out, "abs(")?;
                                self.write_expression_arena(arg, arena, func_info)?;
                                write!(self.out, ")")?;
                            }
                            _ => {
                                return Err(Error::Unsupported("abs for this type".to_string()));
                            }
                        }
                    }
                    Mf::Length => {
                        write!(self.out, "length(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Dot => {
                        let other = arg1.expect("dot requires second argument");
                        write!(self.out, "dot(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(other, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Clamp => {
                        let min_val = arg1.expect("clamp missing min");
                        let max_val = arg2.expect("clamp missing max");
                        write!(self.out, "clamp(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(min_val, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(max_val, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Mix => {
                        let value_b = arg1.expect("mix missing second argument");
                        let factor = arg2.expect("mix missing factor");
                        write!(self.out, "mix(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(value_b, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(factor, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::SmoothStep => {
                        let edge0 = arg1.expect("smoothstep missing edge0");
                        let edge1 = arg2.expect("smoothstep missing edge1");
                        write!(self.out, "smoothstep(")?;
                        self.write_expression_arena(edge0, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(edge1, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Sqrt => {
                        write!(self.out, "std::sqrt(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Cos => {
                        write!(self.out, "std::cos(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Pow => {
                        let exponent = arg1.expect("pow missing exponent");
                        let result_ty = self.resolve_expression_type(handle, func_info);
                        match *result_ty {
                            TypeInner::Scalar(_) => write!(self.out, "std::pow(")?,
                            _ => write!(self.out, "pow(")?,
                        }
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(exponent, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Max => {
                        let rhs = arg1.expect("max missing argument");
                        write!(self.out, "std::max(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(rhs, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    Mf::Min => {
                        let rhs = arg1.expect("min missing argument");
                        write!(self.out, "std::min(")?;
                        self.write_expression_arena(arg, arena, func_info)?;
                        write!(self.out, ", ")?;
                        self.write_expression_arena(rhs, arena, func_info)?;
                        write!(self.out, ")")?;
                    }
                    _ => {
                        return Err(Error::Unsupported(format!(
                            "math function {:?} not supported",
                            fun
                        )));
                    }
                }
            }
            Ex::CallResult(_) => {
                if let Some(name) = self.call_results.get(&handle) {
                    let name = name.clone();
                    write!(self.out, "{}", name)?;
                } else {
                    return Err(Error::Unsupported(
                        "call result used before call statement".to_string(),
                    ));
                }
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
            crate::Literal::F32(value) => {
                let formatted = format!("{:?}", value);
                write!(self.out, "{}f", formatted)?;
            }
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
