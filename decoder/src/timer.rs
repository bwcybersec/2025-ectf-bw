use hal::pac::Tmr0;

/// This is the clock speed of the Timer in Hz
const TIMER_RATE: u32 = 50_000_000;

/// The total allowed transaction time if we detect that we're under attack, in
/// microseconds. This is 5 seconds, per eCTF rules.
const TRANSACTION_TIME_TICKS: u32 = 5 * TIMER_RATE;

/// This type wraps the TMR0 peripheral on the board, allowing us to use it to
/// wait for 5 seconds on a detected attack. It provides functions to start the
/// timer, and to wait until it ends
pub struct DecoderClock {
    pub tmr0: Tmr0,
}

impl DecoderClock {
    pub fn new(tmr0: Tmr0) -> Self {
        Self { tmr0: tmr0 }
    }

    /// Get the current tick count of the timer
    fn now(&self) -> u32 {
        self.tmr0.cnt().read().bits()
    }

    /// Reset and start the transaction timer
    pub fn start_transaction_timer(&self) {
        // The timers aren't implemented in the HAL, so we're setting one up
        // here by hand, following the procedure on page 292 of the User Guide.
        let tmr0 = &self.tmr0;

        // Disable the timer peripheral.
        tmr0.ctrl0()
            .modify(|_, w| w.en_a().clear_bit().en_b().clear_bit());

        while tmr0.ctrl1().read().clken_a().bit_is_set() {}
        while tmr0.ctrl1().read().clken_b().bit_is_set() {}

        // Set the timer source. (50Mhz peripheral clock)

        tmr0.ctrl1().modify(|_, w| {
            // Safety: clksel is 2 bits
            unsafe { w.clksel_a().bits(0b00) }
        });

        // Enable 32-bit cascade mode
        tmr0.ctrl1().modify(|_, w| w.cascade().set_bit());

        // Configure the timer mode (see page 303)
        tmr0.ctrl0().modify(|_, w| {
            w.mode_a()
                .one_shot()
                .clkdiv_a()
                .div_by_1()
                .pol_a()
                .clear_bit()
        });

        // Set the timer compare value (we aren't using the IRQ or Overflow
        // register so we just max this out.)
        tmr0.cmp().write(|w| {
            // Safety: The compare field can take an arbitrary 32-bit number
            unsafe { w.bits(0xFFFFFFFF) }
        });

        // Reset the timer start value.
        tmr0.cnt().write(|w| {
            // Safety: The count field can take an arbitrary 32-bit number
            unsafe { w.bits(1) }
        });

        // Enable the timer clock source
        tmr0.ctrl0().modify(|_, w| w.clken_a().set_bit());

        while tmr0.ctrl1().read().clkrdy_a().bit_is_clear() {}

        // Enable the timer!
        tmr0.ctrl0().modify(|_, w| w.en_a().set_bit());
    }

    /// Wait until we've reached the max transaction time
    pub fn wait_for_max_transaction_time(&self) {
        while self.now() < TRANSACTION_TIME_TICKS {}
    }
}
