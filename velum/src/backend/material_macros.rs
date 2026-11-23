/// Schema with explicitly numbered groups. Entries are the same values as the builder.
#[macro_export]
macro_rules! binding_schema {
    ($($group:expr => [$($entry:expr),* $(,)?]),* $(,)?) => {
        $crate::backend::BindingSchema { groups: vec![$($crate::backend::GroupSchema { index: $group, entries: vec![$($entry),*] }),*] }
    };
}
#[macro_export]
macro_rules! try_binding_set {
    ($(context_id: $context_id:expr,)? group: $group:expr, entries: [$($binding:expr => $resource:expr),* $(,)?] $(,)?) => {{
        let context = None$(.or($context_id))?;
        $crate::try_binding_set!(context: context, group: $group, entries: [$($binding => $resource),*])
    }};

    (context: $context:expr, group: $group:expr, entries: [$($binding:expr => $resource:expr),* $(,)?] $(,)?) => {
        $crate::backend::bindings::try_register($crate::backend::BindingSet {
            context: $context, group: $group, entries: vec![$(($binding, $resource)),*],
        })
    };
}
#[macro_export]
macro_rules! binding_set { ($($tokens:tt)*) => { $crate::try_binding_set!($($tokens)*).expect("Invalid binding set") }; }
/// With named fields, omit the context for the default GPU or supply
/// `context_id: Some(id)`. The original `context` field is also accepted.
#[macro_export]
macro_rules! try_material_template {
    ($description:expr $(,)?) => { $crate::backend::material::try_template($description) };
    ($($field:ident : $value:expr),+ $(,)?) => {
        $crate::try_material_template!(@fields [] $($field: $value,)+)
    };
    (@fields [$($fields:tt)*] context_id: $context:expr, $($rest:tt)*) => {
        $crate::try_material_template!(@register [$($fields)* context: $context,] $($rest)*)
    };
    (@fields [$($fields:tt)*] context: $context:expr, $($rest:tt)*) => {
        $crate::try_material_template!(@register [$($fields)* context: $context,] $($rest)*)
    };
    (@fields [$($fields:tt)*] $field:ident: $value:expr, $($rest:tt)*) => {
        $crate::try_material_template!(@fields [$($fields)* $field: $value,] $($rest)*)
    };
    (@fields [$($fields:tt)*]) => {
        $crate::try_material_template!(@register [$($fields)* context: None,])
    };
    (@register [$($fields:tt)*] $($rest:tt)*) => {
        $crate::backend::material::try_template($crate::backend::MaterialTemplate {
            $($fields)* $($rest)*
        })
    };
}
#[macro_export]
macro_rules! material_template { ($($tokens:tt)*) => { $crate::try_material_template!($($tokens)*).expect("Invalid material template") }; }
/// An optional leading `context_id` checks that the template belongs to that GPU context.
#[macro_export]
macro_rules! try_material {
    (context_id: $context_id:expr, template: $template:expr, bindings: [$($binding:expr),* $(,)?] $(,)?) => {
        $crate::backend::material::try_register_in(
            $crate::backend::Material { template: $template, bindings: vec![$($binding),*] },
            $context_id,
        )
    };
    (template: $template:expr, bindings: [$($binding:expr),* $(,)?] $(,)?) => {
        $crate::backend::material::try_register($crate::backend::Material { template: $template, bindings: vec![$($binding),*] })
    };
}
#[macro_export]
macro_rules! material { ($($tokens:tt)*) => { $crate::try_material!($($tokens)*).expect("Invalid material") }; }
#[macro_export]
macro_rules! try_compute_context {
    ($(context_id: $context_id:expr,)? shader: $shader:expr, schema: $schema:expr $(,)?) => {{
        let context = None$(.or($context_id))?;
        $crate::try_compute_context!(context: context, shader: $shader, schema: $schema)
    }};
    (context: $context:expr, shader: $shader:expr, schema: $schema:expr $(,)?) => {
        $crate::backend::pipeline::try_compute($context, &$shader, &$schema)
    };
}
#[macro_export]
macro_rules! compute_context { ($($tokens:tt)*) => { $crate::try_compute_context!($($tokens)*).expect("Invalid compute context") }; }
