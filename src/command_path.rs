use facet::Facet;
use facet::Peek;

/// Selected CLI command names, excluding option and positional values.
#[derive(Debug)]
pub(crate) struct CommandPath(String);

impl CommandPath {
    pub(crate) fn from_cli<T: Facet<'static>>(cli: &T) -> eyre::Result<Self> {
        let schema = figue::builder::<T>()?.build().schema;
        let mut level = schema.args();
        let mut value = Peek::new(cli);
        let mut names = Vec::new();
        while let Some(field_name) = level.subcommand_field_name() {
            let selected = Self::field(value, field_name)?
                .ok_or_else(|| eyre::eyre!("subcommand field missing from reflected value"))?;
            let selected = if let Ok(optional) = selected.into_option() {
                let Some(selected) = optional.value() else {
                    break;
                };
                selected
            } else {
                selected
            };
            let selected = selected.into_enum()?;
            let variant = selected.active_variant()?;
            let command = level
                .subcommands()
                .values()
                .find(|command| command.effective_name() == variant.effective_name())
                .ok_or_else(|| eyre::eyre!("selected variant missing from CLI schema"))?;
            names.push(command.cli_name());
            level = command.args();
            value = if command.is_flattened_tuple() {
                selected
                    .field(0)?
                    .ok_or_else(|| eyre::eyre!("subcommand payload missing"))?
            } else {
                // Named variant fields are read by the same schema-driven lookup.
                *selected
            };
        }
        Ok(Self(names.join(" ")))
    }

    pub(crate) fn into_string(self) -> String {
        self.0
    }

    fn field<'a>(value: Peek<'a, 'static>, name: &str) -> eyre::Result<Option<Peek<'a, 'static>>> {
        let fields = if let Ok(structure) = value.into_struct() {
            structure.ty().fields
        } else if let Ok(enumeration) = value.into_enum() {
            enumeration.active_variant()?.data.fields
        } else {
            return Ok(None);
        };
        for (index, field) in fields.iter().enumerate() {
            if field.name != name && !field.is_flattened() {
                continue;
            }
            let child = if let Ok(structure) = value.into_struct() {
                structure.field(index)?
            } else {
                value
                    .into_enum()?
                    .field(index)?
                    .ok_or_else(|| eyre::eyre!("variant field missing"))?
            };
            if field.name == name {
                return Ok(Some(child));
            }
            if let Some(found) = Self::field(child, name)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These fixture types form one synthetic CLI schema for reflection tests.
    #[derive(Facet)]
    struct Args {
        #[facet(flatten)]
        commands: Commands,
        #[facet(figue::named)]
        token: String,
    }

    #[derive(Facet)]
    struct Commands {
        #[facet(figue::subcommand, rename = "operation")]
        selected: Option<Operation>,
    }

    #[derive(Facet)]
    #[repr(u8)]
    #[expect(
        dead_code,
        reason = "fixture payloads are read through Facet reflection"
    )]
    enum Operation {
        #[facet(rename = "remote-tools")]
        Remote(Nested),
        Inspect {
            #[facet(figue::named)]
            path: String,
        },
    }

    #[derive(Facet)]
    struct Nested {
        #[facet(figue::subcommand)]
        action: Action,
    }

    #[derive(Facet)]
    #[repr(u8)]
    enum Action {
        #[facet(rename = "show-status")]
        Status,
    }

    #[test]
    fn follows_selected_commands_and_schema_names_without_argument_values() {
        let mut args = Args {
            commands: Commands {
                selected: Some(Operation::Remote(Nested {
                    action: Action::Status,
                })),
            },
            token: "SECRET_ARGUMENT".into(),
        };
        assert_eq!(
            CommandPath::from_cli(&args).unwrap().into_string(),
            "remote-tools show-status"
        );
        args.commands.selected = Some(Operation::Inspect {
            path: "SECRET_PATH".into(),
        });
        assert_eq!(
            CommandPath::from_cli(&args).unwrap().into_string(),
            "inspect"
        );
        args.commands.selected = None;
        assert_eq!(CommandPath::from_cli(&args).unwrap().into_string(), "");
    }
}
