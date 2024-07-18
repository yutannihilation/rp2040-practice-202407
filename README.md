## The pinout of C-551SRD (cathode common):

```
   10 9 8 7 6
   ┌───────┐
   │       │
   │       │
   │       │
   │       │
   │       │
   └───────┘
   1 2 3 4
```

Each pin corresponds to the following positions of the 7 segments.

* Pin 3 and pin 8 are GND
* Pin 5 is not used (the right-bottom period)

```
     ┌─ 7 ─┐
     9     6
     ├─10 ─┤
     1     4
     └─ 2 ─┘
```

## 74HC595

### Wires from RasPi Pico

```
    ┌─────v─────┐
  1 │           │ 16
  2 │           │ 15
  3 │           │ 14 Input  <------------ GPIO2
  4 │           │ 13
  5 │           │ 12 Clock for input  <-- GPIO3
  6 │           │ 11 Clock for output  <- GPIO4
  7 │           │ 10
  8 │           │  9
    └───────────┘
```

### Output

```
     ┌─────v─────┐
QB 1 │           │ 16
QC 2 │           │ 15 QA
QD 3 │           │ 14
QE 4 │           │ 13
QF 5 │           │ 12
QG 6 │           │ 11
QH 7 │           │ 10
   8 │           │  9
     └───────────┘
```

### Other pins

```
      ┌─────v─────┐
    1 │           │ 16 Vcc
    2 │           │ 15 
    3 │           │ 14 Disable  <--- GND
    4 │           │ 13
    5 │           │ 12
    6 │           │ 11
    7 │           │ 10 Clear  <----- Vcc
GND 8 │           │  9 (chain to next)
      └───────────┘
```