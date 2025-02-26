#![no_std]
#![no_main]
// The only reason that this is unstable is because bikeshedding about the zero
// case.
#![feature(array_chunks)]

use crypto::bootstrap_crypto;
use flash::DecoderStorage;
use hal::flc::Flc;
use hal::icc::Icc;
use led::LED;

pub extern crate max7800x_hal as hal;
use decoder::Decoder;
pub use hal::entry;
pub use hal::pac;

use host_comms::DecoderConsole;

use panic_halt as _;

mod cmd_logic;
mod crypto;
mod decoder;
mod flash;
mod host_comms;
mod led;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();

    // Set the system clock to the IPO
    let mut gcr = hal::gcr::Gcr::new(p.gcr, p.lpgcr);
    let ipo = hal::gcr::clocks::Ipo::new(gcr.osc_guards.ipo).enable(&mut gcr.reg);
    let clks = gcr
        .sys_clk
        .set_source(&mut gcr.reg, &ipo)
        .set_divider::<hal::gcr::clocks::Div1>(&mut gcr.reg)
        .freeze();

    // Initialize and split the GPIO0 peripheral into pins
    let gpio0_pins = hal::gpio::Gpio0::new(p.gpio0, &mut gcr.reg).split();
    // Configure UART to host computer with 115200 8N1 settings
    let rx_pin = gpio0_pins.p0_0.into_af1();
    let tx_pin = gpio0_pins.p0_1.into_af1();
    let uart = hal::uart::UartPeripheral::uart0(p.uart0, &mut gcr.reg, rx_pin, tx_pin)
        .baud(115200)
        .clock_pclk(&clks.pclk)
        .parity(hal::uart::ParityBit::None)
        .build();

    // Initialize the GPIO2 peripheral
    let pins = hal::gpio::Gpio2::new(p.gpio2, &mut gcr.reg).split();
    // Enable output mode for the RGB LED pins
    let mut led_r = pins.p2_0.into_input_output();
    let mut led_g = pins.p2_1.into_input_output();
    let mut led_b = pins.p2_2.into_input_output();
    // Use VDDIOH as the power source for the RGB LED pins (3.0V)
    led_r.set_power_vddioh();
    led_g.set_power_vddioh();
    led_b.set_power_vddioh();

    // Set up our abstraction around the LED
    let mut led = LED::new(led_r, led_g, led_b);

    // Set light red: Initializing
    led.red();

    let flc = Flc::new(p.flc, clks.sys_clk);

    // Working with the flash needs the ICC disabled, so we ensure that here
    // We don't need the perfomance boost anyways
    let mut icc = Icc::new(p.icc0);
    icc.disable();

    // Create a new TRNG peripheral instance
    let trng = hal::trng::Trng::new(p.trng, &mut gcr.reg);

    // Initialize our types
    let mut storage = DecoderStorage::init(flc, trng).unwrap();
    let mut decoder = Decoder::new(&mut storage);
    let mut console = DecoderConsole(uart);

    // This preinitializes the VerifyingKey OnceCell, which would
    // otherwise be initialized on the first message received.
    bootstrap_crypto();

    loop {
        // Set light green: Ready!
        led.green();

        if let Err(err) = cmd_logic::run_command(&mut console, &mut decoder, &mut led) {
            err.write_to_console(&console);
        }
    }
}
