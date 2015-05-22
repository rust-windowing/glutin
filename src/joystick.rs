use std::rc::Rc;

/// A Handle to a joystick.
#[derive(Clone)]
pub struct Joystick(::platform::joystick::Joystick);

/// A direction that a hat can point.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Direction {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

#[allow(dead_code)]
enum MaybeCheckout<T> {
    Checkout(::pool::Checkout<T>),
    New(T)
}

impl<T: Clone> Clone for MaybeCheckout<T> {
    fn clone(&self) -> MaybeCheckout<T> {
        match self {
            &MaybeCheckout::Checkout(ref ch) => MaybeCheckout::New((&**ch).clone()),
            &MaybeCheckout::New(ref val) => MaybeCheckout::New((&*val).clone())
        }
    }
}

impl<T> ::std::ops::Deref for MaybeCheckout<T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            &MaybeCheckout::Checkout(ref ch) => &*ch,
            &MaybeCheckout::New(ref data) => data
        }
    }
}

#[derive(Clone)]
pub struct Data {
    axes: MaybeCheckout<Vec<u16>>,
    buttons: MaybeCheckout<Vec<bool>>,
    hats: MaybeCheckout<Vec<Direction>>,
    balls: MaybeCheckout<Vec<(u16, u16)>>,
}

impl Data {
    #[inline]
    pub fn axes(&self) -> &[u16] {
        &self.axes
    }

    #[inline]
    pub fn buttons(&self) -> &[bool] {
        &self.buttons
    }

    #[inline]
    pub fn hats(&self) -> &[Direction] {
        &self.hats
    }

    #[inline]
    pub fn balls(&self) -> &[(u16, u16)] {
        &self.balls
    }
}

/// Describes a joystick.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Description {
    pub axes: usize,
    pub buttons: usize,
    pub hats: usize,
    pub balls: usize,
    pub name: Option<Rc<String>>,
    pub guid: Option<Rc<::uuid::Uuid>>,
}

impl Joystick {
    /// If this returns false, polling for data will always return `None`, and this handle can
    /// safely be dropped.
    pub fn still_attached(&self) -> bool {
        self.0.still_attached()
    }

    /// Poll for the current state of the joystick. This will never block, and will only return
    /// `None` if the joystick has been removed from the system. In the case that there have been
    /// no events on the joystick since the state was last polled, the same state is returned.
    pub fn poll(&self) -> Option<Data> {
        self.0.poll()
    }

    /// Non-blocking iterator over the events of this joystick as they happen. Returns `None` if
    /// the joystick is not attached.
    pub fn events(&self) -> Option<OneEvents> {
        if self.0.still_attached() {
            Some(OneEvents {
                events: ::platform::joystick::OneEvents(&self.0)
            })
        } else {
            None
        }
    }
}

/// A non-blocking iterator over the events of a joystick.
///
/// This is not a "well-behaved" iterator. When `next` returns `None`, this means only that there
/// are no more events currently waiting. Calling `next` again may later return `Some` if more
/// events happen.
pub struct OneEvents<'a> {
    events: ::platform::joystick::OneEvents<'a>
}

impl<'a> Iterator for OneEvents<'a> {
    type Item = OneEvent;
    fn next(&mut self) -> Option<OneEvent> {
        self.events.next()
    }
}

/// An event from a single joystick
pub enum OneEvent {
    /// Joystick was removed from the system.
    Removed,
    /// A button was pressed.
    ButtonDown(usize),
    /// A button was released.
    ButtonUp(usize),
    /// A hat moved to `Direction`.
    Hat(usize, Direction),
    /// A ball moved with `dx` and `dy` being the change along the x and y axis.
    Ball(usize, u16, u16),
    /// An axis changed.
    Axis(usize, u16),
}

pub enum Event {
    /// Joystick was added to the system.
    Added(Joystick),
    /// Joystick was removed from the system.
    Removed(Joystick),
    /// A button was pressed.
    ButtonDown(Joystick, usize),
    /// A button was released.
    ButtonUp(Joystick, usize),
    /// A hat moved to `Direction`.
    Hat(Joystick, usize, Direction),
    /// A ball moved with `dx` and `dy` being the change along the x and y axis.
    Ball(Joystick, usize, u16, u16),
    /// An axis changed.
    Axis(Joystick, usize, u16),
}

/// Has knowledge of all the joysticks on the system, and can provide events on them as well as
/// notify when
pub struct Joysticks {
    joysticks: ::platform::joystick::Joysticks
}

impl Joysticks {
    /// Scan for all joysticks currently attached to the system.
    ///
    /// *Note:* There is an inherent race condition between this method seeing a joystick and the
    /// joystick being unplugged.
    pub fn scan(&self) -> Vec<Joystick> {
        self.joysticks.scan()
    }

    pub fn events(&self) -> Events {
        Events {
            events: ::platform::joystick::Events(&self.joysticks)
        }
    }
}


/// A non-blocking iterator over the events of all joysticks on the system.
///
/// This is not a "well-behaved" iterator. When `next` returns `None`, this means only that there
/// are no more events currently waiting. Calling `next` again may later return `Some` if more
/// events happen.
pub struct Events<'a> {
    events: ::platform::joystick::Events<'a>
}

impl<'a> Iterator for Events<'a> {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        self.events.next()
    }
}
