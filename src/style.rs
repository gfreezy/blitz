use cssparser::{Parser, ParserInput};
use dioxus::core as dioxus_core;
use dioxus_native_core;
use dioxus_native_core::node_ref::{AttributeMask, NodeMask, NodeView};
use dioxus_native_core::state::NodeDepState;
use dioxus_native_core::state::{ParentDepState, State};
use dioxus_native_core_macro::{sorted_str_slice, State};
use parcel_css::properties::border::BorderColor;
use parcel_css::properties::border::BorderSideWidth;
use parcel_css::properties::border::BorderWidth;
use parcel_css::properties::border_radius::BorderRadius;
use parcel_css::traits::Parse;
use parcel_css::values::color::CssColor;
use parcel_css::{properties::Property, stylesheet::ParserOptions};

#[derive(Clone, PartialEq, Debug, State)]
pub(crate) struct Style {
    #[parent_dep_state(color)]
    pub color: ForgroundColor,
    #[node_dep_state()]
    pub bg_color: BackgroundColor,
    #[node_dep_state()]
    pub border: Border,
}

impl Default for Style {
    fn default() -> Self {
        use cssparser::RGBA;
        Style {
            color: ForgroundColor(CssColor::RGBA(RGBA::new(0, 0, 0, 255))),
            bg_color: BackgroundColor(CssColor::RGBA(RGBA::new(255, 255, 255, 0))),
            border: Border::default(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct BackgroundColor(pub CssColor);
impl NodeDepState<()> for BackgroundColor {
    type Ctx = ();

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&["background-color"]));

    fn reduce(&mut self, node: NodeView<'_>, _sibling: (), _: &Self::Ctx) -> bool {
        if let Some(color_attr) = node.attributes().next() {
            if let Some(as_text) = color_attr.value.as_text() {
                let mut value = ParserInput::new(as_text);
                let mut parser = Parser::new(&mut value);
                if let Ok(new_color) = CssColor::parse(&mut parser) {
                    if self.0 != new_color {
                        *self = Self(new_color);
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ForgroundColor(pub CssColor);
impl ParentDepState for ForgroundColor {
    type Ctx = ();
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::new_with_attrs(AttributeMask::Static(&["color"]));

    fn reduce(&mut self, node: NodeView<'_>, parent: Option<&Self>, _: &Self::Ctx) -> bool {
        let new = if let Some(parent) = parent {
            parent.0.clone()
        } else if let Some(color_attr) = node.attributes().next() {
            if let Some(as_text) = color_attr.value.as_text() {
                let mut value = ParserInput::new(as_text);
                let mut parser = Parser::new(&mut value);
                if let Ok(new_color) = CssColor::parse(&mut parser) {
                    new_color
                } else {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        };

        if self.0 != new {
            *self = Self(new);
            true
        } else {
            false
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Border {
    pub colors: BorderColor,
    pub width: BorderWidth,
    pub radius: BorderRadius,
}

impl NodeDepState<()> for Border {
    type Ctx = ();

    const NODE_MASK: NodeMask =
        NodeMask::new_with_attrs(AttributeMask::Static(&sorted_str_slice!([
            "border-color",
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
            "border-radius",
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
            "border-width"
            "border-top-width"
            "border-right-width"
            "border-bottom-width"
            "border-left-width"
        ])));

    fn reduce(&mut self, node: NodeView<'_>, _sibling: (), _: &Self::Ctx) -> bool {
        let mut new = Border::default();
        for a in node.attributes() {
            let mut value = ParserInput::new(a.value.as_text().unwrap());
            let mut parser = Parser::new(&mut value);
            match Property::parse(a.name.into(), &mut parser, &ParserOptions::default()).unwrap() {
                Property::BorderColor(c) => {
                    new.colors = c;
                }
                Property::BorderTopColor(c) => {
                    new.colors.top = c;
                }
                Property::BorderRightColor(c) => {
                    new.colors.right = c;
                }
                Property::BorderBottomColor(c) => {
                    new.colors.bottom = c;
                }
                Property::BorderLeftColor(c) => {
                    new.colors.left = c;
                }
                Property::BorderRadius(r, _) => {
                    new.radius = r;
                }
                Property::BorderTopLeftRadius(r, _) => {
                    new.radius.top_left = r;
                }
                Property::BorderTopRightRadius(r, _) => {
                    new.radius.top_right = r;
                }
                Property::BorderBottomRightRadius(r, _) => {
                    new.radius.bottom_right = r;
                }
                Property::BorderBottomLeftRadius(r, _) => {
                    new.radius.bottom_left = r;
                }
                Property::BorderWidth(width) => {
                    new.width = width;
                }
                Property::BorderTopWidth(width) => {
                    new.width.top = width;
                }
                Property::BorderRightWidth(width) => {
                    new.width.right = width;
                }
                Property::BorderBottomWidth(width) => {
                    new.width.bottom = width;
                }
                Property::BorderLeftWidth(width) => {
                    new.width.left = width;
                }
                _ => {}
            }
        }

        if self != &mut new {
            *self = new;
            true
        } else {
            false
        }
    }
}

impl Default for Border {
    fn default() -> Self {
        Border {
            colors: BorderColor {
                top: CssColor::default(),
                right: CssColor::default(),
                bottom: CssColor::default(),
                left: CssColor::default(),
            },
            radius: BorderRadius::default(),
            width: BorderWidth {
                top: BorderSideWidth::default(),
                right: BorderSideWidth::default(),
                bottom: BorderSideWidth::default(),
                left: BorderSideWidth::default(),
            },
        }
    }
}
