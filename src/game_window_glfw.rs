//! Create window.

// External crates.
use gfx;
use collections::Deque;
use collections::ringbuf::RingBuf;
use glfw;
use glfw::Context;
use gl;
use piston::input;
use piston::input::keyboard;
use piston::input::mouse;
use piston::GameWindow;
use piston::GameWindowSettings;
use piston::shader_version::opengl::OpenGL;

/// Contains stuff for game window.
pub struct GameWindowGLFW {
    /// The window.
    pub window: glfw::Window,
    /// Receives events from window.
    events: Receiver<(f64, glfw::WindowEvent)>,
    /// GLFW context.
    pub glfw: glfw::Glfw,
    /// Game window settings.
    settings: GameWindowSettings,
    event_queue: RingBuf<input::InputEvent>,
    // Used to compute relative mouse movement.
    last_mouse_pos: Option<(f64, f64)>,
}

impl GameWindowGLFW {
    /// Create a new game window from an existing GLFW window.
    pub fn from_pieces(win: glfw::Window, glfw: glfw::Glfw,
                       events: Receiver<(f64, glfw::WindowEvent)>,
                       exit_on_esc: bool) -> GameWindowGLFW {
        win.set_key_polling(true);
        win.set_mouse_button_polling(true);
        win.set_cursor_pos_polling(true);
        win.set_scroll_polling(true);
        win.make_current();

        let (w, h) = win.get_framebuffer_size();
        let fullscreen = win.with_window_mode(|m| match m { glfw::Windowed => true, _ => false });

        GameWindowGLFW {
            window: win,
            events: events,
            glfw: glfw,
            settings: GameWindowSettings {
                title: "<unknown window title, created with from_pieces>".to_string(),
                size: [w as u32, h as u32],
                fullscreen: fullscreen,
                exit_on_esc: exit_on_esc,
            },
            event_queue: RingBuf::new(),
            last_mouse_pos: None,
        }
    }

    /// Creates a new game window for GLFW.
    pub fn new(opengl: OpenGL, settings: GameWindowSettings) -> GameWindowGLFW {
        use glfw::Context;

        // Initialize GLFW.
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        let (major, minor) = opengl.get_major_minor();

        // Make sure we have the right GL version.
        glfw.window_hint(glfw::ContextVersion(major as u32, minor as u32));
        glfw.window_hint(glfw::OpenglForwardCompat(true));
        glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlCoreProfile));

        // Create GLFW window.
        let (window, events) = glfw.create_window(
            settings.size[0],
            settings.size[1],
            settings.title.as_slice(), glfw::Windowed
        ).expect("Failed to create GLFW window.");
        window.set_key_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_scroll_polling(true);
        window.set_char_polling(true);
        // or polling all event
        //window.set_all_polling(true);
        window.make_current();

        // Load the OpenGL function pointers
        gl::load_with(|s| glfw.get_proc_address(s));

        GameWindowGLFW {
            window: window,
            events: events,
            glfw: glfw,
            settings: settings,
            event_queue: RingBuf::new(),
            last_mouse_pos: None,
        }
    }

    fn flush_messages(&mut self) {
        if self.event_queue.len() != 0 {
            return;
        }

        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::KeyEvent(glfw::KeyEscape, _, glfw::Press, _)
                if self.settings.exit_on_esc => {
                    self.window.set_should_close(true);
                }
                glfw::CharEvent(ch) => {
                    self.event_queue.push(input::Text(ch.to_string()));
                }
                glfw::KeyEvent(key, _, glfw::Press, _) => {
                    self.event_queue.push(
                        input::Press(input::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::KeyEvent(key, _, glfw::Release, _) => {
                    self.event_queue.push(
                        input::Release(input::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::MouseButtonEvent(button, glfw::Press, _) => {
                    self.event_queue.push(
                        input::Press(input::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::MouseButtonEvent(button, glfw::Release, _) => {
                    self.event_queue.push(
                        input::Release(input::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::CursorPosEvent(x, y) => {
                    self.event_queue.push(input::Move(input::MouseCursor(x, y)));
                    match self.last_mouse_pos {
                        Some((lx, ly)) => {
                            self.event_queue.push(
                                input::Move(input::MouseRelative(x - lx, y - ly))
                            )
                        }
                        None => {}
                    };
                    self.last_mouse_pos = Some((x, y));
                }
                glfw::ScrollEvent(x, y) => {
                    self.event_queue.push(input::Move(input::MouseScroll(x, y)));
                }
                _ => {}
            }
        }
    }

    /// Creates a gfx device and frame.
    pub fn gfx(&self) -> (gfx::GlDevice, gfx::Frame) {
        let device = gfx::GlDevice::new(|s|
            self.glfw.get_proc_address(s)
        );
        let (w, h) = self.get_size();
        let frame = gfx::Frame::new(w as u16, h as u16);
        (device, frame)
    }
}

impl GameWindow for GameWindowGLFW {
    fn get_settings<'a>(&'a self) -> &'a GameWindowSettings {
        &self.settings
    }

    fn get_draw_size(&self) -> (u32, u32) {
        let (w, h) = self.window.get_framebuffer_size();
        (w as u32, h as u32)
    }

    fn should_close(&self) -> bool {
        self.window.should_close()
    }

    fn close(&mut self) {
        self.window.set_should_close(true);
    }

    fn swap_buffers(&self) {
        use glfw::Context;

        self.window.swap_buffers();
    }

    fn capture_cursor(&mut self, enabled: bool) {
        if enabled {
            self.window.set_cursor_mode(glfw::CursorDisabled)
        } else {
            self.window.set_cursor_mode(glfw::CursorNormal);
            self.last_mouse_pos = None;
        }
    }

    fn poll_event(&mut self) -> Option<input::InputEvent> {
        self.flush_messages();

        if self.event_queue.len() != 0 {
            self.event_queue.pop_front()
        } else {
            None
        }
    }
}

fn glfw_map_key(keycode: glfw::Key) -> keyboard::Key {
    match keycode {
        glfw::Key0 => keyboard::D0,
        glfw::Key1 => keyboard::D1,
        glfw::Key2 => keyboard::D2,
        glfw::Key3 => keyboard::D3,
        glfw::Key4 => keyboard::D4,
        glfw::Key5 => keyboard::D5,
        glfw::Key6 => keyboard::D6,
        glfw::Key7 => keyboard::D7,
        glfw::Key8 => keyboard::D8,
        glfw::Key9 => keyboard::D9,
        glfw::KeyA => keyboard::A,
        glfw::KeyB => keyboard::B,
        glfw::KeyC => keyboard::C,
        glfw::KeyD => keyboard::D,
        glfw::KeyE => keyboard::E,
        glfw::KeyF => keyboard::F,
        glfw::KeyG => keyboard::G,
        glfw::KeyH => keyboard::H,
        glfw::KeyI => keyboard::I,
        glfw::KeyJ => keyboard::J,
        glfw::KeyK => keyboard::K,
        glfw::KeyL => keyboard::L,
        glfw::KeyM => keyboard::M,
        glfw::KeyN => keyboard::N,
        glfw::KeyO => keyboard::O,
        glfw::KeyP => keyboard::P,
        glfw::KeyQ => keyboard::Q,
        glfw::KeyR => keyboard::R,
        glfw::KeyS => keyboard::S,
        glfw::KeyT => keyboard::T,
        glfw::KeyU => keyboard::U,
        glfw::KeyV => keyboard::V,
        glfw::KeyW => keyboard::W,
        glfw::KeyX => keyboard::X,
        glfw::KeyY => keyboard::Y,
        glfw::KeyZ => keyboard::Z,
        glfw::KeyApostrophe => keyboard::Unknown,
        glfw::KeyBackslash => keyboard::Backslash,
        glfw::KeyBackspace => keyboard::Backspace,
        glfw::KeyCapsLock => keyboard::CapsLock,
        glfw::KeyDelete => keyboard::Delete,
        glfw::KeyComma => keyboard::Comma,
        glfw::KeyDown => keyboard::Down,
        glfw::KeyEnd => keyboard::End,
        glfw::KeyEnter => keyboard::Return,
        glfw::KeyEqual => keyboard::Equals,
        glfw::KeyEscape => keyboard::Escape,
        glfw::KeyF1 => keyboard::F1,
        glfw::KeyF2 => keyboard::F2,
        glfw::KeyF3 => keyboard::F3,
        glfw::KeyF4 => keyboard::F4,
        glfw::KeyF5 => keyboard::F5,
        glfw::KeyF6 => keyboard::F6,
        glfw::KeyF7 => keyboard::F7,
        glfw::KeyF8 => keyboard::F8,
        glfw::KeyF9 => keyboard::F9,
        glfw::KeyF10 => keyboard::F10,
        glfw::KeyF11 => keyboard::F11,
        glfw::KeyF12 => keyboard::F12,
        glfw::KeyF13 => keyboard::F13,
        glfw::KeyF14 => keyboard::F14,
        glfw::KeyF15 => keyboard::F15,
        glfw::KeyF16 => keyboard::F16,
        glfw::KeyF17 => keyboard::F17,
        glfw::KeyF18 => keyboard::F18,
        glfw::KeyF19 => keyboard::F19,
        glfw::KeyF20 => keyboard::F20,
        glfw::KeyF21 => keyboard::F21,
        glfw::KeyF22 => keyboard::F22,
        glfw::KeyF23 => keyboard::F23,
        glfw::KeyF24 => keyboard::F24,
        // Possibly next code.
        glfw::KeyF25 => keyboard::Unknown,
        glfw::KeyKp0 => keyboard::NumPad0,
        glfw::KeyKp1 => keyboard::NumPad1,
        glfw::KeyKp2 => keyboard::NumPad2,
        glfw::KeyKp3 => keyboard::NumPad3,
        glfw::KeyKp4 => keyboard::NumPad4,
        glfw::KeyKp5 => keyboard::NumPad5,
        glfw::KeyKp6 => keyboard::NumPad6,
        glfw::KeyKp7 => keyboard::NumPad7,
        glfw::KeyKp8 => keyboard::NumPad8,
        glfw::KeyKp9 => keyboard::NumPad9,
        glfw::KeyKpDecimal => keyboard::NumPadDecimal,
        glfw::KeyKpDivide => keyboard::NumPadDivide,
        glfw::KeyKpMultiply => keyboard::NumPadMultiply,
        glfw::KeyKpSubtract => keyboard::NumPadMinus,
        glfw::KeyKpAdd => keyboard::NumPadPlus,
        glfw::KeyKpEnter => keyboard::NumPadEnter,
        glfw::KeyKpEqual => keyboard::NumPadEquals,
        glfw::KeyLeftShift => keyboard::LShift,
        glfw::KeyLeftControl => keyboard::LCtrl,
        glfw::KeyLeftAlt => keyboard::LAlt,
        glfw::KeyLeftSuper => keyboard::LGui,
        glfw::KeyRightShift => keyboard::RShift,
        glfw::KeyRightControl => keyboard::RCtrl,
        glfw::KeyRightAlt => keyboard::RAlt,
        glfw::KeyRightSuper => keyboard::RGui,
        // Map to backslash?
        glfw::KeyGraveAccent => keyboard::Unknown,
        glfw::KeyHome => keyboard::Home,
        glfw::KeyInsert => keyboard::Insert,
        glfw::KeyLeft => keyboard::Left,
        glfw::KeyLeftBracket => keyboard::LeftBracket,
        glfw::KeyMenu => keyboard::Menu,
        glfw::KeyMinus => keyboard::Minus,
        glfw::KeyNumLock => keyboard::NumLockClear,
        glfw::KeyPageDown => keyboard::PageDown,
        glfw::KeyPageUp => keyboard::PageUp,
        glfw::KeyPause => keyboard::Pause,
        glfw::KeyPeriod => keyboard::Period,
        glfw::KeyPrintScreen => keyboard::PrintScreen,
        glfw::KeyRight => keyboard::Right,
        glfw::KeyRightBracket => keyboard::RightBracket,
        glfw::KeyScrollLock => keyboard::ScrollLock,
        glfw::KeySemicolon => keyboard::Semicolon,
        glfw::KeySlash => keyboard::Slash,
        glfw::KeySpace => keyboard::Space,
        glfw::KeyTab => keyboard::Tab,
        glfw::KeyUp => keyboard::Up,
        glfw::KeyWorld1 => keyboard::Unknown,
        glfw::KeyWorld2 => keyboard::Unknown,
        // _ => keyboard::Unknown,
    }
}

fn glfw_map_mouse(mouse_button: glfw::MouseButton) -> mouse::Button {
    match mouse_button {
        glfw::MouseButton1 => mouse::Left,
        glfw::MouseButton2 => mouse::Right,
        glfw::MouseButton3 => mouse::Middle,
        glfw::MouseButton4 => mouse::X1,
        glfw::MouseButton5 => mouse::X2,
        glfw::MouseButton6 => mouse::Button6,
        glfw::MouseButton7 => mouse::Button7,
        glfw::MouseButton8 => mouse::Button8,
    }
}

