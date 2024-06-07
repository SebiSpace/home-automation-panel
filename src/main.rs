#![no_std]
#![no_main]

mod tasks;
mod setup;

use defmt::*;
use {defmt_rtt as _, panic_halt as _};
use cortex_m_rt::entry;
use cyw43_pio::PioSpi;
use embassy_executor::Executor;
use embassy_rp::{bind_interrupts};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::{PIN_23, PIN_25, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use static_cell::StaticCell;
use crate::tasks::{task_core0, task_core1, WifiPeripherals};

static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static CHANNEL: Channel<CriticalSectionRawMutex, WifiStatus, 1> = Channel::new();

enum WifiStatus {
    Disconnected,
    Connected
}

#[entry]
fn main() -> ! {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());
    let wifi_p = WifiPeripherals{
        pin_23: p.PIN_23, 
        pin_24: p.PIN_24,
        pin_25: p.PIN_25,
        pin_29: p.PIN_29,
        pio0: p.PIO0,
        dma_ch0: p.DMA_CH0
    };

    

    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || { let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| unwrap!(spawner.spawn(task_core1(spawner, wifi_p)))); }
    );

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| unwrap!(spawner.spawn(task_core0(spawner))));
}