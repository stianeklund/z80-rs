# z80-rs

[![Build Status](https://travis-ci.com/stianeklund/z80-rs.svg?branch-master)](https://travis-ci.com/stianeklund/z80-rs)

A WIP Zilog Z80 CPU Emulator

Compatible with Windows, Linux, & Mac OS

## Emulator compatibility

* This is a work in progress project ported from [eighty-eighty](https://github.com/stianeklund/eighty-eighty) and does not run any games, yet.
* Interrupts not implemented.
* Passes the preliminary z80 tests & CPUTEST by SuperSoft Associates.



### CPU Tests

#### Diagnostics II v1.2 by by Supersoft Associates (1981):

```
Test loaded: "CPUTEST.COM" Bytes: 19200

DIAGNOSTICS II V1.2 - CPU TEST
COPYRIGHT (C) 1981 - SUPERSOFT ASSOCIATES

ABCDEFGHIJKLMNOPQRSTUVWXYZ
CPU IS Z80
BEGIN TIMING TEST
END TIMING TEST
CPU TESTS OK
```

#### Preliminary z80 Exerciser (by Frank D. Cringle):

```
Test loaded: "tests/prelim.com" Bytes: 1280
Preliminary tests complete Jump to 0 from 0447
```

#### Preliminary 8080 / z80 Exerciser (by Frank D. Cringle, modified by Ian Bartholemew for the 8080*):
``` 
Test loaded: "8080PRE.COM" Bytes: 1024
8080 Preliminary tests complete
Jump to 0 from 032F
```

#### Zexall

```
*Does not pass.

* See Zexdoc
```
#### Zexdoc

```
Test loaded: "tests/zexdoc.com" Bytes: 8588

Z80doc instruction exerciser
<adc,sbc> hl,<bc,de,hl,sp>....  OK
add hl,<bc,de,hl,sp>..........  OK
add ix,<bc,de,ix,sp>..........  OK
add iy,<bc,de,iy,sp>..........  OK
aluop a,nn....................  ERROR **** crc expected:48799360 found:932ac8f0
^ The rest does not pass
```
--- 

### Arcade game support

Please see [pacman-rs](https://github.com/stianeklund/pacman-rs)

---

#### Running CPU tests:

With Rust & cargo installed:

Run tests from the terminal you can use `cargo test` or, for `stdout` output:
Run all tests: `cargo test -- --nocapture`


---

### References used:

* https://z80.info
* http://www.z80.info/#BASICS_INST
* http://z80.info/zip/z80-documented.pdf
* [Z80 test roms](http://mdfs.net/Software/Z80/Exerciser/)
* https://old.reddit.com/r/emudev & the emudev community on Discord.
