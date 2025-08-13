use crate::GenericResult;
use evdev::uinput::VirtualDevice;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, FFEffectCode, KeyCode, MiscCode, PropType,
    RelativeAxisCode, SwitchCode, UinputAbsSetup,
};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct AbsInfoData {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Capabilities {
    properties: Vec<u16>,
    keys: Vec<u16>,
    relative_axes: Vec<u16>,
    absolute_axes: Vec<(u16, AbsInfoData)>,
    switches: Vec<u16>,
    ff: Vec<u16>,
    max_ff_effects: usize,
    msc: Vec<u16>,
}

pub fn create_device_with_capabilities(
    name: &str,
    capabilities: &Capabilities,
) -> GenericResult<VirtualDevice> {
    let mut device = evdev::uinput::VirtualDeviceBuilder::new()?
        .name(name)
        .with_phys(&CString::new("rs-evdev-uinput")?)?;

    device = device.with_properties(&capabilities.properties())?;

    if let Some(keys) = &capabilities.keys() {
        device = device.with_keys(&keys)?;
    }

    for abs_setup in capabilities.absolute_axes() {
        device = device.with_absolute_axis(&abs_setup)?;
    }

    if let Some(relative_axes) = &capabilities.relative_axes() {
        device = device.with_relative_axes(&relative_axes)?;
    }

    if let Some(switches) = &capabilities.switches() {
        device = device.with_switches(&switches)?;
    }

    if let Some(ff) = &capabilities.ff() {
        device = device.with_ff(&ff)?;
    }

    device = device.with_ff_effects_max(capabilities.max_ff_effects.clone() as u32);

    if let Some(msc) = &capabilities.msc() {
        device = device.with_msc(&msc)?;
    }

    Ok(device.build()?)
}

macro_rules! map_to_primitive {
    ($expression:expr) => {
        $expression
            .map(|v| v.iter().map(|ff| ff.0).collect::<Vec<_>>())
            .unwrap_or_default()
    };
}

pub fn get_capabilities(device: &evdev::Device) -> Capabilities {
    let properties = device
        .properties()
        .iter()
        .map(|prop| prop.0)
        .collect::<Vec<_>>();

    let ff = map_to_primitive!(device.supported_ff());
    let keys = map_to_primitive!(device.supported_keys());
    let msc = map_to_primitive!(device.misc_properties());
    let switches = map_to_primitive!(device.supported_switches());
    let relative_axes = map_to_primitive!(device.supported_relative_axes());

    let abs_info = device.get_absinfo();
    let absolute_axes = if let Ok(abs_info) = abs_info {
        abs_info
            .map(|(code, abs_info)| {
                (
                    code.0,
                    AbsInfoData {
                        value: abs_info.value(),
                        minimum: abs_info.minimum(),
                        maximum: abs_info.maximum(),
                        fuzz: abs_info.fuzz(),
                        flat: abs_info.flat(),
                        resolution: abs_info.resolution(),
                    },
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    Capabilities {
        properties,
        keys,
        relative_axes,
        absolute_axes,
        switches,
        msc,
        ff,
        max_ff_effects: device.max_ff_effects(),
    }
}

macro_rules! map_from_primitive {
    ($name:ident,$t:ident) => {
        fn $name(&self) -> Option<AttributeSet<$t>> {
            let $name = self.$name.iter().map(|&v| $t(v)).collect::<Vec<_>>();
            if $name.is_empty() {
                return None;
            } else {
                return Some(AttributeSet::from_iter($name));
            }
        }
    };
}

impl Capabilities {
    fn properties(&self) -> AttributeSet<PropType> {
        let properties = self
            .properties
            .iter()
            .map(|&prop| PropType(prop))
            .collect::<Vec<_>>();
        AttributeSet::from_iter(properties)
    }

    map_from_primitive!(ff, FFEffectCode);
    map_from_primitive!(keys, KeyCode);
    map_from_primitive!(msc, MiscCode);
    map_from_primitive!(switches, SwitchCode);
    map_from_primitive!(relative_axes, RelativeAxisCode);

    fn absolute_axes(&self) -> Vec<UinputAbsSetup> {
        let absolute_axes = self
            .absolute_axes
            .iter()
            .map(|(code, abs_data)| {
                UinputAbsSetup::new(
                    AbsoluteAxisCode(code.clone()),
                    AbsInfo::new(
                        abs_data.value.clone(),
                        abs_data.minimum.clone(),
                        abs_data.maximum.clone(),
                        abs_data.fuzz.clone(),
                        abs_data.flat.clone(),
                        abs_data.resolution.clone(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        absolute_axes
    }

    pub fn save(&self, path: PathBuf) -> GenericResult<()> {
        let res = serde_json::to_string(self)?;
        fs::write(path, res)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> GenericResult<Self> {
        let contents = fs::read_to_string(path)?;
        let capabilities: Capabilities = serde_json::from_str(&contents)?;
        Ok(capabilities)
    }
}
