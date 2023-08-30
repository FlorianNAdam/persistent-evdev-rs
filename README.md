# Original Program
This project is based on the fantastic [persistent-evdev](https://github.com/aiberia/persistent-evdev) by [aiberia](https://github.com/aiberia).
It was originally just intended to be a simple port, but I ended up rewriting the program from scratch.


# Theory of Operation
Disconnecting evdev devices, such as with USB hotplug, can be problematic when using QEMU+Libvirt since
it will not reopen the device when the device reconnects. Furthermore, existing libvirt hotplug
mechanisms don't handle qemu:commandline options or cgroup dev permissions well.

A workaround is to create a udev proxy device and pass that virtual device to QEMU instead. This script
works by detecting input device hotplug, automatically opening the device, and proxying the
key/mouse events to the virtual udev device.

The capabilities of each physical device is cached so that the virtual udev devices can be created
appropriately even when the real devices don't exist.

# Setup

TODO

Edit the config file (`/opt/persistent-evdev-rs/etc/config.json`) to point to your actual devices:

```json
{
    "cache": "/opt/persistent-evdev-rs/cache",
    "devices": {
        "persist-mouse0": "/dev/input/by-id/usb-Logitech_G403_Prodigy_Gaming_Mouse_078738533531-event-if01",
        "persist-mouse1": "/dev/input/by-id/usb-Logitech_G403_Prodigy_Gaming_Mouse_078738533531-event-mouse",
        "persist-mouse2": "/dev/input/by-id/usb-Logitech_G403_Prodigy_Gaming_Mouse_078738533531-if01-event-kbd",
        "persist-keyboard0": "/dev/input/by-id/usb-Microsoft_Natural®_Ergonomic_Keyboard_4000-event-kbd",
        "persist-keyboard1": "/dev/input/by-id/usb-Microsoft_Natural®_Ergonomic_Keyboard_4000-if01-event-kbd"
    }
}
```

Restart udev for the new rules, then enable and start the persistent-evdev service:

```shell
systemctl restart systemd-udevd
systemctl enable --now persistent-evdev-rs.service
```

If everything worked new devices should be created as such:

```shell
$ ls -al /dev/input/by-id/
total 0
drwxr-xr-x 2 root root 140 Aug 17 02:23 .
drwxr-xr-x 4 root root 280 Aug 18 16:15 ..
lrwxrwxrwx 1 root root  11 Aug 17 02:23 uinput-persist-keyboard0 -> ../event256
lrwxrwxrwx 1 root root  11 Aug 17 02:23 uinput-persist-keyboard1 -> ../event257
lrwxrwxrwx 1 root root  10 Aug 17 02:23 uinput-persist-mouse0 -> ../event29
lrwxrwxrwx 1 root root  10 Aug 17 02:23 uinput-persist-mouse1 -> ../event30
lrwxrwxrwx 1 root root  10 Aug 17 02:23 uinput-persist-mouse2 -> ../event31
```

# Example usage with Libvirt

Add the virtual devices to `cgroup_device_acl` in `/etc/libvirt/qemu.conf` as such:
```
cgroup_device_acl = [
    "/dev/null", "/dev/full", "/dev/zero",
    "/dev/random", "/dev/urandom",
    "/dev/ptmx", "/dev/kvm",
    "/dev/input/by-id/uinput-persist-keyboard0",
    "/dev/input/by-id/uinput-persist-keyboard1",
    "/dev/input/by-id/uinput-persist-mouse0",
    "/dev/input/by-id/uinput-persist-mouse1",
    "/dev/input/by-id/uinput-persist-mouse2"
]
```

Add the virtual devices to your VM XML:
```xml
<qemu:commandline>
    <qemu:arg value='-object'/>
    <qemu:arg value='input-linux,id=input1,evdev=/dev/input/by-id/uinput-persist-keyboard0,grab_all=on,repeat=on'/>
    <qemu:arg value='-object'/>
    <qemu:arg value='input-linux,id=input2,evdev=/dev/input/by-id/uinput-persist-keyboard1'/>
    <qemu:arg value='-object'/>
    <qemu:arg value='input-linux,id=input3,evdev=/dev/input/by-id/uinput-persist-mouse0'/>
    <qemu:arg value='-object'/>
    <qemu:arg value='input-linux,id=input4,evdev=/dev/input/by-id/uinput-persist-mouse1'/>
    <qemu:arg value='-object'/>
    <qemu:arg value='input-linux,id=input5,evdev=/dev/input/by-id/uinput-persist-mouse2'/>
</qemu:commandline>
```

See the usual Libvirt Evdev guides for more details.