mod network;

use cyw43::Runner;
use cyw43_pio::PioSpi;
use defmt::{info, unwrap};
use embassy_time::Timer;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use static_cell::StaticCell;
use crate::tasks::network::{net_task, wifi_task};

const WIFI_NETWORK: &str = "spacestationWIFI";
const WIFI_PASSWORD: &str = "orange-rocket-bad!737MAX";

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub(crate) struct WifiPeripherals {
    pub pin_23: PIN_23,
    pub pin_24: PIN_24,
    pub pin_25: PIN_25,
    pub pin_29: PIN_29,
    pub pio0: PIO0,
    pub dma_ch0: DMA_CH0
}


#[embassy_executor::task]
pub(crate) async fn task_core0(spawner: Spawner) {
    info!("Hello from core 0");
    loop {
        Timer::after_millis(100).await;
    }
}


#[embassy_executor::task]
pub(crate) async fn task_core1(
    spawner: Spawner,
    p: WifiPeripherals
) {
    info!("Hello from core 1");

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.pin_23, Level::Low);
    let cs = Output::new(p.pin_25, Level::High);
    let mut pio = Pio::new(p.pio0, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, pio.irq0, cs, p.pin_24, p.pin_29, p.dma_ch0);

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    unwrap!(spawner.spawn(wifi_task(runner)));
    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<2>::new()),
        seed,
    ));

    unwrap!(spawner.spawn(net_task(stack)));
    
    // put scanning logic here for wifi discovery on (first?) startup.

    loop {
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }
    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    // And now we can use it!
}