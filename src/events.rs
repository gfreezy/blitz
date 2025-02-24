use piet_wgpu::kurbo::Point;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use taffy::prelude::Size;
use tao::event::MouseButton;

use dioxus::{
    core::{ElementId, EventPriority, Mutations, UserEvent},
    events::{KeyboardData, MouseData},
    prelude::dioxus_elements::{
        geometry::{
            euclid::Point2D, ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint,
        },
        input_data::{self, keyboard_types::Modifiers, MouseButtonSet},
    },
};
use dioxus_native_core::real_dom::NodeType;

use tao::keyboard::Key;

use crate::{focus::FocusState, mouse::get_hovered, node::PreventDefault, Dom, TaoEvent};

const DBL_CLICK_TIME: Duration = Duration::from_millis(500);

struct CursorState {
    position: Coordinates,
    buttons: MouseButtonSet,
    last_click: Option<Instant>,
    last_pressed_element: Option<ElementId>,
    last_clicked_element: Option<ElementId>,
    hovered: Option<ElementId>,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            position: Coordinates::new(
                Point2D::default(),
                Point2D::default(),
                Point2D::default(),
                Point2D::default(),
            ),
            buttons: Default::default(),
            last_click: Default::default(),
            last_pressed_element: Default::default(),
            last_clicked_element: Default::default(),
            hovered: Default::default(),
        }
    }
}

#[derive(Default)]
struct EventState {
    modifier_state: Modifiers,
    cursor_state: CursorState,
    focus_state: Arc<Mutex<FocusState>>,
}

#[derive(Default)]
pub struct BlitzEventHandler {
    state: EventState,
    queued_events: Vec<UserEvent>,
}

impl BlitzEventHandler {
    pub(crate) fn new(focus_state: Arc<Mutex<FocusState>>) -> Self {
        Self {
            state: EventState {
                focus_state,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub(crate) fn register_event(
        &mut self,
        event: &TaoEvent,
        rdom: &mut Dom,
        viewport_size: &Size<u32>,
    ) {
        match event {
            tao::event::Event::NewEvents(_) => (),
            tao::event::Event::WindowEvent {
                window_id: _,
                event,
                ..
            } => {
                match event {
                    tao::event::WindowEvent::Resized(_) => (),
                    tao::event::WindowEvent::Moved(_) => (),
                    tao::event::WindowEvent::CloseRequested => (),
                    tao::event::WindowEvent::Destroyed => (),
                    tao::event::WindowEvent::DroppedFile(_) => (),
                    tao::event::WindowEvent::HoveredFile(_) => (),
                    tao::event::WindowEvent::HoveredFileCancelled => (),
                    tao::event::WindowEvent::ReceivedImeText(_) => (),
                    tao::event::WindowEvent::Focused(_) => (),
                    tao::event::WindowEvent::KeyboardInput {
                        device_id: _,
                        event,
                        is_synthetic: _,
                        ..
                    } => {
                        let key = serde_json::to_value(&event.logical_key).unwrap();
                        let code = event.physical_key.to_string();

                        let data = KeyboardData::new(
                            serde_json::from_value(key).unwrap(),
                            input_data::keyboard_types::Code::from_str(&code).unwrap(),
                            match event.location {
                                tao::keyboard::KeyLocation::Standard => {
                                    input_data::keyboard_types::Location::Standard
                                }
                                tao::keyboard::KeyLocation::Left => {
                                    input_data::keyboard_types::Location::Left
                                }
                                tao::keyboard::KeyLocation::Right => {
                                    input_data::keyboard_types::Location::Right
                                }
                                tao::keyboard::KeyLocation::Numpad => {
                                    input_data::keyboard_types::Location::Numpad
                                }
                                _ => todo!(),
                            },
                            event.repeat,
                            self.state.modifier_state,
                        );

                        // keypress events are only triggered when a key that has text is pressed
                        if let tao::event::ElementState::Pressed = event.state {
                            if event.text.is_some() {
                                self.queued_events.push(UserEvent {
                                    scope_id: None,
                                    priority: EventPriority::Medium,
                                    element: Some(ElementId(1)),
                                    name: "keypress",
                                    data: Arc::new(data.clone()),
                                    bubbles: true,
                                });
                            }
                            if let Key::Tab = event.logical_key {
                                self.state.focus_state.lock().unwrap().progress(
                                    rdom,
                                    !self.state.modifier_state.contains(Modifiers::SHIFT),
                                );
                                return;
                            }
                        }

                        self.queued_events.push(UserEvent {
                            scope_id: None,
                            priority: EventPriority::Medium,
                            element: self.state.focus_state.lock().unwrap().last_focused_id,
                            name: match event.state {
                                tao::event::ElementState::Pressed => "keydown",
                                tao::event::ElementState::Released => "keyup",
                                _ => todo!(),
                            },
                            data: Arc::new(data),
                            bubbles: true,
                        });
                    }
                    tao::event::WindowEvent::ModifiersChanged(mods) => {
                        let mut modifiers = Modifiers::empty();
                        if mods.alt_key() {
                            modifiers |= Modifiers::ALT;
                        }
                        if mods.control_key() {
                            modifiers |= Modifiers::CONTROL;
                        }
                        if mods.super_key() {
                            modifiers |= Modifiers::META;
                        }
                        if mods.shift_key() {
                            modifiers |= Modifiers::SHIFT;
                        }
                        self.state.modifier_state = modifiers;
                    }
                    tao::event::WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                        ..
                    } => {
                        let pos = Point::new(position.x as f64, position.y as f64);
                        let hovered = get_hovered(rdom, viewport_size, pos);
                        let (mouse_x, mouse_y) = (pos.x as i32, pos.y as i32);
                        let screen_point = ScreenPoint::new(mouse_x as f64, mouse_y as f64);
                        let client_point = ClientPoint::new(mouse_x as f64, mouse_y as f64);
                        let page_point = PagePoint::new(mouse_x as f64, mouse_y as f64);
                        // the position of the element is subtracted later
                        let element_point = ElementPoint::new(mouse_x as f64, mouse_y as f64);
                        let position =
                            Coordinates::new(screen_point, client_point, element_point, page_point);

                        let data = MouseData::new(
                            Coordinates::new(screen_point, client_point, element_point, page_point),
                            None,
                            self.state.cursor_state.buttons,
                            self.state.modifier_state,
                        );
                        match (hovered, self.state.cursor_state.hovered) {
                            (Some(hovered), Some(old_hovered)) => {
                                if hovered != old_hovered {
                                    self.queued_events.push(UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        element: Some(hovered),
                                        name: "mouseenter",
                                        data: Arc::new(data.clone()),
                                        bubbles: true,
                                    });
                                    self.queued_events.push(UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        element: Some(old_hovered),
                                        name: "mouseleave",
                                        data: Arc::new(data),
                                        bubbles: true,
                                    });
                                    self.state.cursor_state.hovered = Some(hovered);
                                }
                            }
                            (Some(hovered), None) => {
                                self.queued_events.push(UserEvent {
                                    scope_id: None,
                                    priority: EventPriority::Medium,
                                    element: Some(hovered),
                                    name: "mouseenter",
                                    data: Arc::new(data),
                                    bubbles: true,
                                });
                                self.state.cursor_state.hovered = Some(hovered);
                            }
                            (None, Some(old_hovered)) => {
                                self.queued_events.push(UserEvent {
                                    scope_id: None,
                                    priority: EventPriority::Medium,
                                    element: Some(old_hovered),
                                    name: "mouseleave",
                                    data: Arc::new(data),
                                    bubbles: true,
                                });
                                self.state.cursor_state.hovered = None;
                            }
                            (None, None) => (),
                        }
                        self.state.cursor_state.position = position;
                    }
                    tao::event::WindowEvent::CursorEntered { device_id: _ } => (),
                    tao::event::WindowEvent::CursorLeft { device_id: _ } => (),
                    tao::event::WindowEvent::MouseWheel {
                        device_id: _,
                        delta: _,
                        phase: _,
                        ..
                    } => (),
                    tao::event::WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                        ..
                    } => {
                        if let Some(hovered) = self.state.cursor_state.hovered {
                            let button = match button {
                                MouseButton::Left => input_data::MouseButton::Primary,
                                MouseButton::Middle => input_data::MouseButton::Auxiliary,
                                MouseButton::Right => input_data::MouseButton::Secondary,
                                MouseButton::Other(num) => match num {
                                    4 => input_data::MouseButton::Fourth,
                                    5 => input_data::MouseButton::Fifth,
                                    _ => input_data::MouseButton::Unknown,
                                },
                                _ => input_data::MouseButton::Unknown,
                            };

                            match state {
                                tao::event::ElementState::Pressed => {
                                    self.state.cursor_state.buttons |= button;
                                }
                                tao::event::ElementState::Released => {
                                    self.state.cursor_state.buttons.remove(button);
                                }
                                _ => todo!(),
                            }

                            let pos = &self.state.cursor_state.position;

                            let data = MouseData::new(
                                Coordinates::new(
                                    pos.screen(),
                                    pos.client(),
                                    pos.element(),
                                    pos.page(),
                                ),
                                None,
                                self.state.cursor_state.buttons,
                                self.state.modifier_state,
                            );

                            let prevent_default = &rdom[hovered].state.prevent_default;
                            match state {
                                tao::event::ElementState::Pressed => {
                                    self.queued_events.push(UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        element: Some(hovered),
                                        name: "mousedown",
                                        data: Arc::new(data),
                                        bubbles: true,
                                    });
                                    self.state.cursor_state.last_pressed_element = Some(hovered);
                                }
                                tao::event::ElementState::Released => {
                                    self.queued_events.push(UserEvent {
                                        scope_id: None,
                                        priority: EventPriority::Medium,
                                        element: Some(hovered),
                                        name: "mouseup",
                                        data: Arc::new(data.clone()),
                                        bubbles: true,
                                    });

                                    // click events only trigger if the mouse button is pressed and released on the same element
                                    if self.state.cursor_state.last_pressed_element.take()
                                        == Some(hovered)
                                    {
                                        self.queued_events.push(UserEvent {
                                            scope_id: None,
                                            priority: EventPriority::Medium,
                                            element: Some(hovered),
                                            name: "click",
                                            data: Arc::new(data.clone()),
                                            bubbles: true,
                                        });

                                        if let Some(last_clicked) =
                                            self.state.cursor_state.last_click.take()
                                        {
                                            if self.state.cursor_state.last_clicked_element
                                                == Some(hovered)
                                                && last_clicked.elapsed() < DBL_CLICK_TIME
                                            {
                                                self.queued_events.push(UserEvent {
                                                    scope_id: None,
                                                    priority: EventPriority::Medium,
                                                    element: Some(hovered),
                                                    name: "dblclick",
                                                    data: Arc::new(data),
                                                    bubbles: true,
                                                });
                                            }
                                        }

                                        self.state.cursor_state.last_clicked_element =
                                            Some(hovered);
                                        self.state.cursor_state.last_click = Some(Instant::now());
                                    }
                                }
                                _ => todo!(),
                            }
                            if *prevent_default != PreventDefault::MouseUp
                                && rdom[hovered].state.focus.level.focusable()
                            {
                                self.state
                                    .focus_state
                                    .lock()
                                    .unwrap()
                                    .set_focus(rdom, hovered);
                            }
                        }
                    }
                    tao::event::WindowEvent::TouchpadPressure {
                        device_id: _,
                        pressure: _,
                        stage: _,
                    } => (),
                    tao::event::WindowEvent::AxisMotion {
                        device_id: _,
                        axis: _,
                        value: _,
                    } => (),
                    tao::event::WindowEvent::Touch(_) => (),
                    tao::event::WindowEvent::ScaleFactorChanged {
                        scale_factor: _,
                        new_inner_size: _,
                    } => (),
                    tao::event::WindowEvent::ThemeChanged(_) => (),
                    tao::event::WindowEvent::DecorationsClick => (),
                    _ => (),
                }
            }
            tao::event::Event::DeviceEvent {
                device_id: _,
                event: _,
                ..
            } => (),
            tao::event::Event::UserEvent(_) => (),
            tao::event::Event::MenuEvent {
                window_id: _,
                menu_id: _,
                origin: _,
                ..
            } => (),
            tao::event::Event::TrayEvent {
                bounds: _,
                event: _,
                position: _,
                ..
            } => (),
            tao::event::Event::GlobalShortcutEvent(_) => (),
            tao::event::Event::Suspended => (),
            tao::event::Event::Resumed => (),
            tao::event::Event::MainEventsCleared => (),
            tao::event::Event::RedrawRequested(_) => (),
            tao::event::Event::RedrawEventsCleared => (),
            tao::event::Event::LoopDestroyed => (),
            _ => (),
        }
    }

    pub fn drain_events(&mut self) -> Vec<UserEvent> {
        let mut events = Vec::new();
        std::mem::swap(&mut self.queued_events, &mut events);
        events
    }

    fn prune_id(&mut self, removed: ElementId) {
        if let Some(id) = self.state.cursor_state.hovered {
            if id == removed {
                self.state.cursor_state.hovered = None;
            }
        }
        if let Some(id) = self.state.cursor_state.last_pressed_element {
            if id == removed {
                self.state.cursor_state.last_pressed_element = None;
            }
        }
        if let Some(id) = self.state.cursor_state.last_clicked_element {
            if id == removed {
                self.state.cursor_state.last_clicked_element = None;
            }
        }
    }

    pub(crate) fn prune(&mut self, mutations: &Mutations, rdom: &Dom) {
        fn remove_children(handler: &mut BlitzEventHandler, rdom: &Dom, removed: ElementId) {
            handler.prune_id(removed);
            if let NodeType::Element { children, .. } = &rdom[removed].node_type {
                for child in children {
                    remove_children(handler, rdom, *child);
                }
            }
        }
        for m in &mutations.edits {
            match m {
                dioxus::core::DomEdit::ReplaceWith { root, .. } => {
                    remove_children(self, rdom, ElementId(*root as usize))
                }
                dioxus::core::DomEdit::Remove { root } => {
                    remove_children(self, rdom, ElementId(*root as usize))
                }
                _ => (),
            }
        }
    }

    pub(crate) fn clean(&self) -> bool {
        self.state.focus_state.lock().unwrap().clean()
    }
}
