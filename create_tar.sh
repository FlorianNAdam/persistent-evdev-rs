#!/bin/bash

VERSION="0.1.0"

tar -cvf persistent-evdev-rs-$VERSION-x86_64.tar LICENSE.md
tar -rvf persistent-evdev-rs-$VERSION-x86_64.tar config.json
tar -rvf persistent-evdev-rs-$VERSION-x86_64.tar -C target/release persistent-evdev-rs
tar -rvf persistent-evdev-rs-$VERSION-x86_64.tar -C systemd/ persistent-evdev-rs.service
tar -rvf persistent-evdev-rs-$VERSION-x86_64.tar -C udev/ 60-persistent-input-rs-uinput.rules
gzip -f persistent-evdev-rs-$VERSION-x86_64.tar
sha256sum persistent-evdev-rs-$VERSION-x86_64.tar.gz