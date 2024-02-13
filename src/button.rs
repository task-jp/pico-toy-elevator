use embedded_hal::digital::v2::{InputPin, OutputPin};

pub trait LedButtonTrait {
    fn is_pressed(&self) -> Option<bool>;
    fn turn_on(&mut self) -> Option<()>;
    fn turn_off(&mut self) -> Option<()>;
}

pub struct LedButton<LED, BUTTON> {
    led: LED,
    button: BUTTON,
}

impl<LED, BUTTON> LedButton<LED, BUTTON>
where
    LED: OutputPin,
    BUTTON: InputPin,
{
    pub fn new(led: LED, button: BUTTON) -> Self {
        Self { led, button }
    }
}

impl<LED, BUTTON> LedButtonTrait for LedButton<LED, BUTTON>
where
    LED: OutputPin,
    BUTTON: InputPin,
{
    fn is_pressed(&self) -> Option<bool> {
        self.button.is_low().ok()
    }

    fn turn_on(&mut self) -> Option<()> {
        self.led.set_low().ok()
    }

    fn turn_off(&mut self) -> Option<()> {
        self.led.set_high().ok()
    }
}
