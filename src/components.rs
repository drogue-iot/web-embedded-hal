use core::future::Future;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll};
use embedded_hal_async::digital::Wait;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub struct WebLed {
    pin: &'static OutputPin,
}

impl WebLed {
    pub fn new(pin: &'static OutputPin) -> Self {
        Self { pin }
    }
}

impl embedded_hal::digital::v2::OutputPin for WebLed {
    type Error = ();
    fn set_high(&mut self) -> Result<(), ()> {
        self.pin.set_value(true);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), ()> {
        self.pin.set_value(false);
        Ok(())
    }
}

pub struct WebButton {
    pin: &'static InputPin,
}

impl WebButton {
    pub fn new(pin: &'static InputPin) -> Self {
        Self { pin }
    }
}

impl embedded_hal::digital::v2::InputPin for WebButton {
    type Error = ();
    fn is_high(&self) -> Result<bool, ()> {
        Ok(self.pin.get_value())
    }
    fn is_low(&self) -> Result<bool, ()> {
        Ok(self.pin.get_value())
    }
}

use core::convert::Infallible;
impl embedded_hal_1::digital::ErrorType for WebButton {
    type Error = Infallible;
}

impl Wait for WebButton {
    type WaitForHighFuture<'m> = SignalFuture<'m, ()>;
    fn wait_for_high<'m>(&'m mut self) -> Self::WaitForHighFuture<'m> {
        self.pin.wait()
    }

    type WaitForLowFuture<'m> = SignalFuture<'m, ()>;
    fn wait_for_low<'m>(&'m mut self) -> Self::WaitForLowFuture<'m> {
        self.pin.wait()
    }
    type WaitForRisingEdgeFuture<'m> = SignalFuture<'m, ()>;
    fn wait_for_rising_edge<'m>(&'m mut self) -> Self::WaitForRisingEdgeFuture<'m> {
        self.pin.wait()
    }
    type WaitForFallingEdgeFuture<'m> = SignalFuture<'m, ()>;
    fn wait_for_falling_edge<'m>(&'m mut self) -> Self::WaitForFallingEdgeFuture<'m> {
        self.pin.wait()
    }

    type WaitForAnyEdgeFuture<'m> = SignalFuture<'m, ()>;
    fn wait_for_any_edge<'m>(&'m mut self) -> Self::WaitForAnyEdgeFuture<'m> {
        self.pin.wait()
    }
}

pub struct InputPin {
    high: AtomicBool,
    signal: Signal<()>,
}

impl InputPin {
    pub const fn new() -> Self {
        Self {
            high: AtomicBool::new(true),
            signal: Signal::new(),
        }
    }

    pub fn configure(&'static mut self, element: &str) {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");

        let cb = Closure::wrap(Box::new(move || {
            if self.high.load(Ordering::Acquire) {
                self.high.store(false, Ordering::Release);
            } else {
                self.high.store(true, Ordering::Release);
            }
            self.signal.signal(());
        }) as Box<dyn FnMut()>);

        let val = document
            .get_element_by_id(element)
            .expect("unable to find element");
        val.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
            .expect("error adding event listener");
        cb.forget();
    }

    fn get_value(&self) -> bool {
        self.high.load(Ordering::Acquire)
    }

    fn wait<'a>(&'a self) -> SignalFuture<'a, ()> {
        self.signal.reset();
        SignalFuture::new(&self.signal)
    }
}

pub struct OutputPin {
    high: AtomicBool,
    element: MaybeUninit<&'static str>,
    transform: MaybeUninit<fn(bool) -> OutputVisual>,
}

impl OutputPin {
    pub const fn new() -> Self {
        Self {
            high: AtomicBool::new(false),
            element: MaybeUninit::uninit(),
            transform: MaybeUninit::uninit(),
        }
    }

    pub fn configure(
        &'static mut self,
        element: &'static str,
        transform: fn(bool) -> OutputVisual,
    ) {
        unsafe {
            let p = self.element.as_mut_ptr();
            p.write(element);
            let f = self.transform.as_mut_ptr();
            f.write(transform);
        };
        self.set_value(false);
    }

    pub fn set_value(&self, high: bool) {
        let (element, transform) = unsafe { (&*self.element.as_ptr(), &*self.transform.as_ptr()) };
        self.high.store(high, Ordering::Release);
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let txt = document.get_element_by_id(element).unwrap();
        let output = transform(high);
        txt.set_inner_html(output.as_ref());
    }
}

pub struct SignalFuture<'s, T: Send> {
    signal: &'s Signal<T>,
}

impl<'s, T: Send> SignalFuture<'s, T> {
    pub fn new(signal: &'s Signal<T>) -> Self {
        Self { signal }
    }
}

impl<T: Send> Future for SignalFuture<'_, T> {
    type Output = Result<T, Infallible>;
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = self.signal.poll_wait(cx);
        if let Poll::Ready(r) = result {
            Poll::Ready(Ok(r))
        } else {
            Poll::Pending
        }
    }
}

/// A led color.
///
/// Can be converted into a output visual by adding the boolean state:
/// ```rust
/// use drogue_wasm::{OutputVisual, LedColor};
///
/// let visual: OutputVisual = true + LedColor::Red;
/// let transformer: fn(bool) -> OutputVisual = |state| state + LedColor::Red;
/// ```
pub enum LedColor {
    Red,
    Green,
    Yellow,
    Orange,
    Blue,
}

impl std::ops::Add<LedColor> for bool {
    type Output = OutputVisual;

    fn add(self, rhs: LedColor) -> Self::Output {
        OutputVisual::Led(rhs, self)
    }
}

impl std::ops::Add<bool> for LedColor {
    type Output = OutputVisual;

    fn add(self, rhs: bool) -> Self::Output {
        OutputVisual::Led(self, rhs)
    }
}

pub enum OutputVisual {
    String(&'static str),
    Led(LedColor, bool),
}

macro_rules! to_led {
    ($state:expr, $on:literal, $off:literal) => {
        match $state {
            true => concat!(
                r#"<svg height="50" width="50"><circle cx="25" cy="25" r="10" fill=""#,
                $on,
                r#"" stroke-width="3" stroke=""#,
                $off,
                r#""/></svg>"#
            ),
            false => concat!(
                r#"<svg height="50" width="50"><circle cx="25" cy="25" r="10" fill=""#,
                $off,
                r#"" /></svg>"#
            ),
        }
    };
}

impl AsRef<str> for OutputVisual {
    fn as_ref(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Led(c, state) => match c {
                LedColor::Red => to_led!(state, "red", "darkred"),
                LedColor::Green => to_led!(state, "lightgreen", "darkgreen"),
                LedColor::Yellow => to_led!(state, "yellow", "#94a000"),
                LedColor::Orange => to_led!(state, "orange", "#a97200"),
                LedColor::Blue => to_led!(state, "lightblue", "darkblue"),
            },
        }
    }
}

impl From<&'static str> for OutputVisual {
    fn from(s: &'static str) -> Self {
        OutputVisual::String(s)
    }
}

impl From<bool> for OutputVisual {
    fn from(v: bool) -> Self {
        match v {
            true => OutputVisual::String("ON"),
            false => OutputVisual::String("OFF"),
        }
    }
}

/// Signal type from embassy, using critical sections to provide a thread safe signalling mechanism
use core::cell::UnsafeCell;
use core::mem;
use core::task::Waker;

/// Synchronization primitive. Allows creating awaitable signals that may be passed between tasks.
/// For a simple use-case where the receiver is only ever interested in the latest value of
/// something, Signals work well. For more advanced use cases, you might want to use [`Channel`](crate::channel::channel::Channel) instead..
///
/// Signals are generally declared as being a static const and then borrowed as required.
pub struct Signal<T> {
    state: UnsafeCell<State<T>>,
}

enum State<T> {
    None,
    Waiting(Waker),
    Signaled(T),
}

unsafe impl<T: Send> Send for Signal<T> {}
unsafe impl<T: Send> Sync for Signal<T> {}

impl<T> Signal<T> {
    pub const fn new() -> Self {
        Self {
            state: UnsafeCell::new(State::None),
        }
    }
}

impl<T: Send> Signal<T> {
    /// Mark this Signal as completed.
    pub fn signal(&self, val: T) {
        critical_section::with(|_| unsafe {
            let state = &mut *self.state.get();
            if let State::Waiting(waker) = mem::replace(state, State::Signaled(val)) {
                waker.wake();
            }
        })
    }

    pub fn reset(&self) {
        critical_section::with(|_| unsafe {
            let state = &mut *self.state.get();
            *state = State::None
        })
    }

    pub fn poll_wait(&self, cx: &mut Context<'_>) -> Poll<T> {
        critical_section::with(|_| unsafe {
            let state = &mut *self.state.get();
            match state {
                State::None => {
                    *state = State::Waiting(cx.waker().clone());
                    Poll::Pending
                }
                State::Waiting(w) if w.will_wake(cx.waker()) => Poll::Pending,
                State::Waiting(_) => panic!("waker overflow"),
                State::Signaled(_) => match mem::replace(state, State::None) {
                    State::Signaled(res) => Poll::Ready(res),
                    _ => unreachable!(),
                },
            }
        })
    }

    /// Future that completes when this Signal has been signaled.
    pub fn wait(&self) -> impl Future<Output = T> + '_ {
        futures::future::poll_fn(move |cx| self.poll_wait(cx))
    }

    /// non-blocking method to check whether this signal has been signaled.
    pub fn signaled(&self) -> bool {
        critical_section::with(|_| matches!(unsafe { &*self.state.get() }, State::Signaled(_)))
    }
}
