use hal::gpio::{InputOutput, Pin};

/// Reprsentation of the RGB LED, giving it some functions to set the color
pub struct Led {
    led_r: Pin<2, 0, InputOutput>,
    led_g: Pin<2, 1, InputOutput>,
    led_b: Pin<2, 2, InputOutput>,
}

impl Led {
    pub fn new(
        led_r: Pin<2, 0, InputOutput>,
        led_g: Pin<2, 1, InputOutput>,
        led_b: Pin<2, 2, InputOutput>,
    ) -> Self {
        Self {
            led_r,
            led_g,
            led_b,
        }
    }

    fn set_lights(&mut self, red: bool, green: bool, blue: bool) {
        if red {
            self.led_r.set_low();
        } else {
            self.led_r.set_high();
        }

        if green {
            self.led_g.set_low();
        } else {
            self.led_g.set_high();
        }

        if blue {
            self.led_b.set_low();
        } else {
            self.led_b.set_high();
        }
    }

    pub fn red(&mut self) {
        self.set_lights(true, false, false);
    }

    pub fn green(&mut self) {
        self.set_lights(false, true, false);
    }

    pub fn cyan(&mut self) {
        self.set_lights(false, true, true);
    }

    pub fn magenta(&mut self) {
        self.set_lights(true, false, true);
    }

    pub fn yellow(&mut self) {
        self.set_lights(true, true, false);
    }
}
