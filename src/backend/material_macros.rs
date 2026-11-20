/// Schema with explicitly numbered groups. Entries are the same values as the builder.
#[macro_export]
macro_rules! binding_schema {
    ($($group:expr => [$($entry:expr),* $(,)?]),* $(,)?) => {
        $crate::backend::BindingSchema { groups: vec![$($crate::backend::GroupSchema { index: $group, entries: vec![$($entry),*] }),*] }
    };
}
#[macro_export]
macro_rules! try_binding_set {
    (context: $context:expr, group: $group:expr, entries: [$($binding:expr => $resource:expr),* $(,)?] $(,)?) => {
        $crate::backend::bindings::try_register($crate::backend::BindingSet {
            context: $context, group: $group, entries: vec![$(($binding, $resource)),*],
        })
    };
}
#[macro_export]
macro_rules! binding_set { ($($tokens:tt)*) => { $crate::try_binding_set!($($tokens)*).expect("Invalid binding set") }; }
/// All template fields (including shader entries, layouts and raster state) remain accessible.
#[macro_export]
macro_rules! try_material_template {
    ($description:expr $(,)?) => { $crate::backend::material::try_template($description) };
    ($($field:ident : $value:expr),* $(,)?) => { $crate::backend::material::try_template($crate::backend::MaterialTemplate { $($field: $value),* }) };
}
#[macro_export]
macro_rules! material_template { ($($tokens:tt)*) => { $crate::try_material_template!($($tokens)*).expect("Invalid material template") }; }
#[macro_export]
macro_rules! try_material {
    (template: $template:expr, bindings: [$($binding:expr),* $(,)?] $(,)?) => {
        $crate::backend::material::try_register($crate::backend::Material { template: $template, bindings: vec![$($binding),*] })
    };
}
#[macro_export]
macro_rules! material { ($($tokens:tt)*) => { $crate::try_material!($($tokens)*).expect("Invalid material") }; }
#[macro_export]
macro_rules! try_compute_context {
    (context: $context:expr, shader: $shader:expr, schema: $schema:expr $(,)?) => {
        $crate::backend::pipeline::try_compute($context, &$shader, &$schema)
    };
}
#[macro_export]
macro_rules! compute_context { ($($tokens:tt)*) => { $crate::try_compute_context!($($tokens)*).expect("Invalid compute context") }; }
