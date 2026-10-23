//! Input state: which keys and mouse buttons are held, pressed or released this frame,
//! and how far the mouse moved.
//!
//! The event loop feeds winit events in as they arrive; the frame's update reads them;
//! [`InputState::end_frame`] clears the per-frame parts. Keys are physical
//! ([`KeyCode`]), so `W A S D` stay in the same place on every keyboard layout.

use std::collections::HashSet;
use std::hash::Hash;

use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

/// Held / pressed / released sets for one kind of button.
///
/// The sets are cleared, never dropped, so after the first few presses no frame
/// allocates.
#[derive(Debug)]
struct Buttons<T> {
    held: HashSet<T>,
    pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T: Copy + Eq + Hash> Buttons<T> {
    fn new() -> Self {
        Self {
            held: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }

    fn event(&mut self, button: T, state: ElementState) {
        match state {
            // A press of a held button (key repeat) isn't a new press.
            ElementState::Pressed => {
                if self.held.insert(button) {
                    self.pressed.insert(button);
                }
            }
            // A release of a button that isn't held (pressed before the window had
            // focus, or consumed by the client) is ignored.
            ElementState::Released => {
                if self.held.remove(&button) {
                    self.released.insert(button);
                }
            }
        }
    }

    /// Releases every held button, as if each one was let go this frame.
    fn release_all(&mut self) {
        self.released.extend(self.held.drain());
    }

    fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

/// The keyboard and mouse as the current frame sees them.
#[derive(Debug)]
pub struct InputState {
    keys: Buttons<KeyCode>,
    mouse: Buttons<MouseButton>,
    mouse_delta: (f64, f64),
}

impl InputState {
    /// Nothing held, no movement.
    pub fn new() -> Self {
        Self {
            keys: Buttons::new(),
            mouse: Buttons::new(),
            mouse_delta: (0.0, 0.0),
        }
    }

    /// Records a key press or release. Repeated presses of a held key are ignored.
    pub fn key(&mut self, key: KeyCode, state: ElementState) {
        self.keys.event(key, state);
    }

    /// Records a mouse button press or release.
    pub fn mouse_button(&mut self, button: MouseButton, state: ElementState) {
        self.mouse.event(button, state);
    }

    /// Adds raw mouse movement (`DeviceEvent::MouseMotion`: unaccelerated device units,
    /// not pixels).
    pub fn mouse_motion(&mut self, (dx, dy): (f64, f64)) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    /// The window lost focus: every held key and button counts as released this frame,
    /// and this frame's mouse movement is dropped. The window sees no releases while
    /// unfocused, so otherwise a key let go elsewhere would stay held.
    pub fn focus_lost(&mut self) {
        self.keys.release_all();
        self.mouse.release_all();
        self.mouse_delta = (0.0, 0.0);
    }

    /// Clears what happened this frame (presses, releases, mouse movement). Held keys
    /// and buttons stay held. Call once at the end of every frame.
    pub fn end_frame(&mut self) {
        self.keys.end_frame();
        self.mouse.end_frame();
        self.mouse_delta = (0.0, 0.0);
    }

    /// Whether `key` is down.
    pub fn held(&self, key: KeyCode) -> bool {
        self.keys.held.contains(&key)
    }

    /// Whether `key` went down this frame.
    pub fn pressed(&self, key: KeyCode) -> bool {
        self.keys.pressed.contains(&key)
    }

    /// Whether `key` went up this frame.
    pub fn released(&self, key: KeyCode) -> bool {
        self.keys.released.contains(&key)
    }

    /// Whether mouse `button` is down.
    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse.held.contains(&button)
    }

    /// Whether mouse `button` went down this frame.
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse.pressed.contains(&button)
    }

    /// Whether mouse `button` went up this frame.
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse.released.contains(&button)
    }

    /// Raw mouse movement this frame, `(right, down)` in device units.
    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use winit::event::ElementState::{Pressed, Released};
    use winit::event::MouseButton;
    use winit::keyboard::KeyCode;

    use super::InputState;

    #[test]
    fn press_hold_release() {
        let mut input = InputState::new();
        input.key(KeyCode::KeyW, Pressed);
        assert!(input.pressed(KeyCode::KeyW));
        assert!(input.held(KeyCode::KeyW));
        assert!(!input.released(KeyCode::KeyW));

        input.end_frame();
        assert!(!input.pressed(KeyCode::KeyW));
        assert!(input.held(KeyCode::KeyW));

        input.key(KeyCode::KeyW, Released);
        assert!(input.released(KeyCode::KeyW));
        assert!(!input.held(KeyCode::KeyW));

        input.end_frame();
        assert!(!input.released(KeyCode::KeyW));
        assert!(!input.held(KeyCode::KeyA));
    }

    #[test]
    fn tap_within_one_frame_is_pressed_and_released() {
        let mut input = InputState::new();
        input.key(KeyCode::Space, Pressed);
        input.key(KeyCode::Space, Released);
        assert!(input.pressed(KeyCode::Space));
        assert!(input.released(KeyCode::Space));
        assert!(!input.held(KeyCode::Space));
    }

    #[test]
    fn key_repeat_is_not_a_new_press() {
        let mut input = InputState::new();
        input.key(KeyCode::KeyA, Pressed);
        input.end_frame();
        input.key(KeyCode::KeyA, Pressed);
        assert!(!input.pressed(KeyCode::KeyA));
        assert!(input.held(KeyCode::KeyA));
    }

    #[test]
    fn release_without_press_is_ignored() {
        let mut input = InputState::new();
        input.mouse_button(MouseButton::Left, Released);
        assert!(!input.mouse_released(MouseButton::Left));
        assert!(!input.mouse_held(MouseButton::Left));
    }

    #[test]
    fn mouse_buttons() {
        let mut input = InputState::new();
        input.mouse_button(MouseButton::Right, Pressed);
        assert!(input.mouse_pressed(MouseButton::Right));
        assert!(input.mouse_held(MouseButton::Right));
        assert!(!input.mouse_held(MouseButton::Left));
        input.end_frame();
        input.mouse_button(MouseButton::Right, Released);
        assert!(input.mouse_released(MouseButton::Right));
        assert!(!input.mouse_held(MouseButton::Right));
    }

    #[test]
    fn mouse_motion_adds_up_within_a_frame() {
        let mut input = InputState::new();
        input.mouse_motion((3.0, -1.0));
        input.mouse_motion((2.0, 4.5));
        assert_eq!(input.mouse_delta(), (5.0, 3.5));
        input.end_frame();
        assert_eq!(input.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn focus_loss_releases_everything() {
        let mut input = InputState::new();
        input.key(KeyCode::KeyW, Pressed);
        input.key(KeyCode::ShiftLeft, Pressed);
        input.mouse_button(MouseButton::Left, Pressed);
        input.mouse_motion((10.0, 0.0));
        input.end_frame();

        input.focus_lost();
        assert!(!input.held(KeyCode::KeyW));
        assert!(!input.held(KeyCode::ShiftLeft));
        assert!(!input.mouse_held(MouseButton::Left));
        assert!(input.released(KeyCode::KeyW));
        assert!(input.released(KeyCode::ShiftLeft));
        assert!(input.mouse_released(MouseButton::Left));
        assert_eq!(input.mouse_delta(), (0.0, 0.0));

        // The key comes back up while unfocused; the window never hears of it.
        input.end_frame();
        input.key(KeyCode::KeyW, Pressed);
        assert!(input.pressed(KeyCode::KeyW));
    }
}
