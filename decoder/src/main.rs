#![no_std]
#![no_main]
// The only reason that this is unstable is because bikeshedding about the zero
// case.
#![feature(array_chunks)]

extern crate alloc;
use flash::DecoderStorage;
use hal::flc::Flc;
use hal::icc::Icc;

use core::ptr::addr_of_mut;

pub extern crate max7800x_hal as hal;
use decoder::Decoder;
pub use hal::entry;
pub use hal::pac;

use host_comms::DecoderConsole;

use panic_halt as _;
// use panic_semihosting as _;

use embedded_alloc::LlffHeap as Heap;

mod cmd_logic;
mod crypto;
mod decoder;
mod flash;
mod host_comms;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[entry]
fn main() -> ! {
    // Allocate a very silly 4k of heap for formatting error and debug strings :)
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 4096;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    // heprintln!("Hello from semihosting!");
    let p = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut gcr = hal::gcr::Gcr::new(p.gcr, p.lpgcr);
    let ipo = hal::gcr::clocks::Ipo::new(gcr.osc_guards.ipo).enable(&mut gcr.reg);
    let clks = gcr
        .sys_clk
        .set_source(&mut gcr.reg, &ipo)
        .set_divider::<hal::gcr::clocks::Div1>(&mut gcr.reg)
        .freeze();

    // Initialize a delay timer using the ARM SYST (SysTick) peripheral
    let rate = clks.sys_clk.frequency;
    let mut delay = cortex_m::delay::Delay::new(core.SYST, rate);

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

    // uart.write_bytes(b"Hello, world!\r\n");

    // heprintln!("LEDs should be on");
    // Initialize the GPIO2 peripheral
    let pins = hal::gpio::Gpio2::new(p.gpio2, &mut gcr.reg).split();
    // Enable output mode for the RGB LED pins
    let mut led_r = pins.p2_0.into_input_output();
    let mut led_g = pins.p2_1.into_input_output();
    let mut led_b = pins.p2_2.into_input_output();
    // Use VDDIOH as the power source for the RGB LED pins (3.0V)
    // Note: This HAL API may change in the future
    led_r.set_power_vddioh();
    led_g.set_power_vddioh();
    led_b.set_power_vddioh();


    let flc = Flc::new(p.flc, clks.sys_clk);

    let mut icc = Icc::new(p.icc0);

    icc.disable();
    // heprintln!("Initializing decoder storage.");
    let mut storage = DecoderStorage::init(flc).unwrap();

    // heprintln!("Initializing decoder.");
    let mut decoder: Decoder<'_> = Decoder::new(&mut storage);
    // dbg!(&decoder);

    let mut console = DecoderConsole(uart);

    led_r.set_high();
    // led_g.set_high();
    led_b.set_high();
    delay.delay_ms(500);

    loop {
        if let Err(err) = cmd_logic::run_command(&mut console, &mut decoder) {
            err.write_to_console(&console);
        }
    }
}
