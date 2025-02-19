use core::sync::atomic::{AtomicU32, Ordering};

use cortex_m::delay::Delay;
use fugit::{ExtU32, Instant};
use hal::pac::{Tmr0, SYST};
use cortex_m_rt::interrupt;
use hal::Interrupt as interrupt;

/// The total allowed transaction time if we detect that we're under attack, in
/// microseconds. This is 5 seconds, per eCTF rules.
const TRANSACTION_TIME_MILLIS: u32 = 5000;

pub type ClockInstant = Instant<u32, 100, 1>;


/// This is a counter of how many times we've gotten the timer interrupt, which
/// is set up to occur once every 10ms.
static TICKS: AtomicU32 = AtomicU32::new(0);

pub struct DecoderClock {
    tmr0: Tmr0,
    transaction_time: Option<ClockInstant>
}

#[interrupt]
fn TMR0() {
    TICKS.fetch_add(1, Ordering::Relaxed);

    // Reset the TMR0 IRQ 
    // Safety: no other code references the TMR0 IRQ field.
    unsafe { 
        let tmr0 = Tmr0::steal();
        tmr0.intfl().write(|w| {
            w.irq_a().clear_bit()
        });
    } 
}

/// This type keeps track of the time, allows us to wait for a specific amount 
/// of time, and allows us to get a representation of a moment in time. It also
/// keeps track of the state of one "transaction", allowing us to wait until
/// that transaction has hit the maximum transaction time.
///
/// This implementation has 10ms accuracy.
impl DecoderClock {
    pub fn init(mut tmr0: Tmr0) -> Self {
        // The timers aren't implemented in the HAL, so we're setting one up
        // here by hand, following the procedure on page 292 of the User Guide.
        
        // Disable the timer peripheral.
        tmr0.ctrl0().write(|w| {
            w.en_a().clear_bit()            
        });

        while tmr0.ctrl1().read().clken_a().bit_is_set() {}

        // Set the timer source.
        //
        // We're using the INRO because it's already enabled and set to 30kHz,
        // which is a nice value for us.

        tmr0.ctrl1().write(|w| {
            // 2 is INRO for TMR0
            // Safety: 2 is a known good value for clksel_a
            unsafe {w.clksel_a().bits(2)}
        });

        // Configure the timer for continuous mode (see page 303)
        tmr0.ctrl0().write(|w| {
            w
            .mode_a().continuous()
            .clkdiv_a().div_by_1()
        });

        // Enable the timer interrupt.
        tmr0.ctrl1().write(|w| {
            w.ie_a().set_bit()
        });

        // Set the timer compare value: 300 is once every 10ms at 30kHz
        tmr0.cmp().write(|w| {
            // Safety: The compare field can take an arbitrary 32-bit number
            unsafe { w.compare().bits(300) }
        });

        // Enable the timer clock source
        tmr0.ctrl0().write(|w| {
            w.clken_a().set_bit()
        });

        while tmr0.ctrl1().read().clkrdy_a().bit_is_set() {}

        // Enable the timer!
        tmr0.ctrl0().write(|w| {
            w.en_a().set_bit()
        });

        while tmr0.ctrl1().read().clken_a().bit_is_set() {}

        Self {
            tmr0,
            transaction_time: None
        }
    }

    fn get_cur_tick(&self) -> u32 {
        TICKS.load(Ordering::Relaxed)
    }

    /// Get an instant representing the current time
    pub fn now(&self) -> ClockInstant {
        ClockInstant::from_ticks(self.get_cur_tick())
    }

    /// Wait for a particular instant.
    pub fn wait_for_instant(&self, instant: ClockInstant) {
        while self.now() < instant {}
    }

    /// Start the transaction timer
    pub fn start_transaction(&mut self) {
        self.transaction_time = Some(self.now());
    }

    /// Wait until we've reached the max transaction time
    pub fn wait_for_max_transaction_time(&self) {
        if let Some(transaction_start) = self.transaction_time {
            let transaction_end = transaction_start + TRANSACTION_TIME_MILLIS.millis();
            self.wait_for_instant(transaction_end);
        }
    }
}