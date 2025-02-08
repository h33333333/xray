#[macro_export]
macro_rules! render_order_enum {
    ( $name:ident, $( $variant:ident ),+ ) => {
        /// Represents the currently active element and the order in which this set of elements is rendered.
        ///
        /// The order in which variants are defined represents the order in which the corresponding elements will be rendered.
        #[derive(Debug, Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
        pub enum $name {
            #[default]
            $(
                $variant,
            )*
        }

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                match value {
                    $(
                      $name::$variant => ${index()},
                    )*
                }
            }
        }

        impl TryFrom<usize> for $name {
            type Error = anyhow::Error;

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                Ok(match value {
                    $(
                        ${index()} => $name::$variant,
                    )*
                    _ => anyhow::bail!("no matching variant of {} found for the provided index", stringify!($name)),
                })
            }
        }

        impl $name {
            const NUMBER_OF_FIELDS: usize = ${count($variant)};

            pub fn toggle(&mut self, direction: $crate::tui::action::Direction) {
                let current_variant_idx = Into::<usize>::into(*self);

                let next_variant_idx = match direction {
                    $crate::tui::action::Direction::Forward => (current_variant_idx + 1) % Self::NUMBER_OF_FIELDS,
                    $crate::tui::action::Direction::Backward => (current_variant_idx + Self::NUMBER_OF_FIELDS - 1) % Self::NUMBER_OF_FIELDS,
                };

                if let Ok(next_variant) = $name::try_from(next_variant_idx) {
                    *self = next_variant;
                } else {
                    // This is pretty much uncreacheable, as macro ensures that the `next_variant_idx` is always valid.
                    // Still, I don't want the app to panic in this case. A simple log is enough.
                    tracing::debug!("Failed to toggle the currently active element for {}. Unknown index: {}", stringify!($name), next_variant_idx);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! sort_fields_by_render_order {
    ( $order_enum:ident ) => {
        impl $order_enum {
            fn sort_fields_by_order(fields: &mut [$crate::tui::view::pane::util::Field<'_, $order_enum>]) {
                fields.sort_by(|(a, _), (b, _)| a.cmp(b))
            }
        }
    };
}
