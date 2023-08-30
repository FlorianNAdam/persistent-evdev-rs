extern crate core;

mod capabilities;

use evdev;
use evdev::uinput::VirtualDevice;

use crate::capabilities::{create_device_with_capabilities, Capabilities};
use clap::Parser;
use evdev::InputEvent;
use lazy_static::lazy_static;
use log::{error, info, LevelFilter};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use udev::{EventType, MonitorBuilder};

pub type GenericResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

lazy_static! {
    pub static ref CONFIG: Config = Config::new().unwrap_or_else(|e| {
        println!("failed loading config: {}", e);
        exit(1);
    });
}

#[derive(Parser)]
struct Args {
    config_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_cache")]
    cache: PathBuf,
    devices: HashMap<String, PathBuf>,
    #[serde(default = "default_udev_interval")]
    udev_interval: u64,
}

fn default_cache() -> PathBuf {
    PathBuf::from("/opt/persistent-evdev-rs/cache")
}

fn default_udev_interval() -> u64 {
    50
}

impl Config {
    fn new() -> GenericResult<Self> {
        let args = Args::parse();
        let path = args.config_path;
        let contents = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&contents)?)
    }
}

struct Device {
    name: String,
    path: PathBuf,
    uinput: Option<VirtualDevice>,
}

impl Device {
    fn new(name: String, path: PathBuf) -> Self {
        let capabilities_path = CONFIG.cache.join(&name).with_extension("json");
        let capabilities = Capabilities::load(capabilities_path).ok();

        let uinput = if let Some(capabilities) = &capabilities {
            let device = create_device_with_capabilities(&name, capabilities)
                .expect("failed creating uinput device");
            Some(device)
        } else {
            None
        };

        Self { name, path, uinput }
    }

    async fn open_uinput(&mut self, evdev: &evdev::Device) {
        if self.uinput.is_none() {
            let capabilities = capabilities::get_capabilities(evdev);

            if capabilities
                .save(CONFIG.cache.join(&self.name).with_extension("json"))
                .is_err()
            {
                error!("failed saving capabilities for {}", self.name);
            }

            if let Ok(device) = create_device_with_capabilities(&self.name, &capabilities) {
                self.uinput.replace(device);
            }
        }
    }
}

struct State {
    tx: UnboundedSender<Device>,
    rx: UnboundedReceiver<Device>,
}

impl State {
    fn new() -> Self {
        let (tx, rx) = unbounded_channel();

        for (device_name, device_path) in CONFIG.devices.iter() {
            let device = Device::new(device_name.clone(), device_path.clone());
            let _ = tx.send(device);
            info!("created uinput device: {}", device_name);
        }

        Self { tx, rx }
    }
}

async fn release(device: &mut evdev::Device) -> GenericResult<()> {
    let key_state = device.get_key_state()?;
    let mut keys = vec![];
    for value in key_state.iter() {
        keys.push(format!("{:?}", value));
        device.send_events(&vec![InputEvent::new_now(1, value.0, 1)][..])?;
    }
    Ok(())
}

async fn wait_for_release(device: &evdev::Device) -> GenericResult<()> {
    while device.get_key_state()?.iter().next().is_some() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

async fn grab(device: &mut evdev::Device) -> GenericResult<()> {
    release(device).await?;
    wait_for_release(device).await?;
    device.grab()?;
    Ok(())
}

async fn event_proxy(mut evdev: evdev::Device, uinput: &mut VirtualDevice) -> GenericResult<()> {
    grab(&mut evdev).await?;

    loop {
        let mut events = vec![];
        for event in evdev.fetch_events()? {
            events.push(event);
        }
        let _ = uinput.emit(&events.as_slice());
    }
}

async fn evdev_thread(mut device: Device, evdev: evdev::Device, tx: UnboundedSender<Device>) {
    info!("opened evdev {} {:?}", device.name, device.path);

    if let Some(uinput) = &mut device.uinput {
        let _ = event_proxy(evdev, uinput).await;
        info!("closed evdev {} {:?}", device.name, device.path)
    }

    let _ = tx.send(device);
}

async fn update_devices(state: &mut State) {
    while let Ok(mut device) = state.rx.try_recv() {
        if device.path.exists() {
            if let Ok(evdev) = evdev::Device::open(&device.path) {
                device.open_uinput(&evdev).await;

                tokio::spawn(evdev_thread(device, evdev, state.tx.clone()));
                continue;
            }
        }
        let _ = state.tx.send(device);
    }
}

async fn udev_loop(mut state: State) {
    let builder = MonitorBuilder::new().expect("failed creating udev monitor");
    let builder = builder
        .match_subsystem("input")
        .expect("failed finding input subsystem");
    let monitor = builder.listen().expect("failed binding to udev events");

    loop {
        for event in monitor.iter() {
            if event.event_type() == EventType::Add {
                update_devices(&mut state).await;
            }
        }
        sleep(Duration::from_millis(CONFIG.udev_interval.clone())).await;
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    SimpleLogger::new()
        .env()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let mut state = State::new();

    update_devices(&mut state).await;

    udev_loop(state).await;
}
