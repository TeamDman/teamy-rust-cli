use facet::Type;
use facet::UserType;

#[must_use]
pub fn to_kebab_case(name: &str) -> String {
    let mut out = String::new();
    let mut previous_is_alphanumeric = false;

    for character in name.chars() {
        if character == '_' {
            out.push('-');
            previous_is_alphanumeric = false;
            continue;
        }

        if character.is_ascii_uppercase() {
            if previous_is_alphanumeric {
                out.push('-');
            }
            out.push(character.to_ascii_lowercase());
            previous_is_alphanumeric = true;
            continue;
        }

        out.push(character);
        previous_is_alphanumeric = character.is_ascii_alphanumeric();
    }

    out
}

#[must_use]
pub fn normalize_command_token(token: &str) -> String {
    token.replace('_', "-").to_ascii_lowercase()
}

#[must_use]
pub fn unwrap_option_shape(mut shape: &'static facet::Shape) -> &'static facet::Shape {
    while let Ok(option_def) = shape.def.into_option() {
        shape = option_def.t;
    }
    shape
}

#[must_use]
pub fn shape_struct_fields(shape: &'static facet::Shape) -> Option<&'static [facet::Field]> {
    let shape = unwrap_option_shape(shape);
    match shape.ty {
        Type::User(UserType::Struct(struct_type)) => Some(struct_type.fields),
        _ => None,
    }
}

#[must_use]
pub fn shape_enum_variants(shape: &'static facet::Shape) -> Option<&'static [facet::Variant]> {
    let shape = unwrap_option_shape(shape);
    match shape.ty {
        Type::User(UserType::Enum(enum_type)) => Some(enum_type.variants),
        _ => None,
    }
}

#[must_use]
pub fn field_is_bool_flag(field: &facet::Field) -> bool {
    let shape = unwrap_option_shape(field.shape());
    shape.type_identifier.eq_ignore_ascii_case("bool")
}
