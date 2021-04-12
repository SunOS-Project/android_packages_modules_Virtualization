# Microdroid

Microdroid is a (very) lightweight version of Android that is intended to run on
on-device virtual machines. It is built from the same source code as the regular
Android, but it is much smaller; no system server, no HALs, no GUI, etc. It is
intended to host headless & native workloads only.

## Building

You need a VIM3L board. Instructions for building Android for the target, and
flashing the image can be found [here](../docs/getting_started/yukawa.md).

Then you build microdroid. Note that the instruction below is very likely to
change in the future, because this is in active development. For example, the
`microdroid_*` modules will eventually be included in the `com.android.virt`
APEX, which is already in the `yukawa` (VIM3L) target.

```
$ source build/envsetup.sh
$ choosecombo 1 aosp_arm64 userdebug // actually, any arm64-based target is ok
$ m microdroid_super
$ m microdroid_boot-5.10
$ m microdroid_vendor_boot-5.10
$ m microdroid_bootloader
$ m microdroid_uboot_env
$ m microdroid_vbmeta
$ m microdroid_vbmeta_system
$ m microdroid_cdisk.json
```

## Installing

Push the built files to the device. In addition to that, some other files have
to be manually created, for now. In the future, you won't need these.

```
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_bootloader /data/local/tmp/bootloader
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_super.img /data/local/tmp/super.img
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_boot-5.10.img /data/local/tmp/boot.img
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_vendor_boot-5.10.img /data/local/tmp/vendor_boot.img
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_vbmeta.img /data/local/tmp/vbmeta.img
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_vbmeta_system.img /data/local/tmp/vbmeta_system.img
$ adb push $ANDROID_PRODUCT_OUT/system/etc/uboot_env.img /data/local/tmp
$ adb push $ANDROID_PRODUCT_OUT/system/etc/microdroid_cdisk.json /data/local/tmp
$ dd if=/dev/zero of=misc.img bs=4k count=256
$ adb push misc.img /data/local/tmp/
```

## Running

Create the composite image using `assemble_cvd` and run it via `crosvm`. In the
future, this shall be done via [`virtmanager`](../virtmanager/).

```
$ adb shell 'cd /data/local/tmp; /apex/com.android.virt/bin/mk_cdisk microdroid_cdisk.json os_composite.img'
$ adb shell 'cd /data/local/tmp; /apex/com.android.virt/bin/crosvm run --cid=5 --disable-sandbox --bios=bootloader --serial=type=stdout --disk=os_composite.img'
```

The CID in `--cid` parameter can be anything greater than 2 (`VMADDR_CID_HOST`).

## ADB

```
$ adb forward tcp:8000 vsock:5:5555
$ adb connect localhost:8000
```

`5` in `vsock:5` should match with the CID number that was given to `crosvm`.
`5555` must be the value. `8000` however can be any port in the development
machine.

Done. Now you can log into microdroid. Have fun!

```
$ adb -s localhost:8000 shell
```
