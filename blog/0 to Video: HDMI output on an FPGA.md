## Introduction

Back in my first year of university I took a course on digital logic. The homework was tedious as you might expect: doing boolean arithmetic by hand, drawing state machine diagrams, using Karnaugh maps, etc. The weekly lab group projects turned out to be pretty cool though. We built circuits with a drag and drop tool in Quartus, verified them with ModelSim, and ran them on an Altera DE FPGA development board. There were some components we never got to using like VGA output. 

Thinking I could do some cool projects, I bought an [FPGA board from Kickstarter](https://www.kickstarter.com/projects/201461162/maxprologic-ultra-low-cost-fpga-development-board/faqs) on a whim. It's been gathering dust for some time now so I set my mind to trying to build a little video display project. There were some interesting posts on hackaday like one where [Wenting Zhang uses an FPGA to display realtime ASCII art](https://hackaday.io/project/66319-realtime-vga-ascii-art-converter/), but I was nowhere near that level.

I first needed to switch from using block diagrams in Quartus to using a hardware description language. We briefly discussed VHDL in my digital logic class, so I wrote a few lines of VHDL to blink the board's LEDs as a hello world. I had trouble declaring sequential logic in VHDL and switched to Verilog HDL since the board's developer had an example in it. I went through [Deepak's Verilog tutorial on asic-world.com](http://www.asic-world.com/verilog/veritut.html) and tried a few blinking patterns like this strobing effect:

<video controls src="/files/blinky.mp4"></video>

I set my sights on a simpler yet equally daunting project: *outputting video over HDMI*.

## No HDMI output? No problem.

My past self made the unfortunate oversight of getting an FPGA board without an HDMI port so I needed to buy an HDMI breakout board. Adafruit only had the PCB version in stock at the time so I also bought some soldering equipment.

### Amazon
* Hakko FX-901 cordless soldering iron $28.50
* Eneloop batteries with charger $17.99
* Lead-free rosin core solder $9.59
* Lead-free tip tinner $6.50

Total: $62.58

### Adafruit
* 20 Male-Male jumper wires $1.95
* 20 Female-Male jumper wires $1.95
* Panel mount HDMI socket breakout $3.50

Total: $7.40

My Amazon order took only a day to arrive. I chose the cheapest shipping option on Adafruit so I had to wait another week to get started.

The goods (excluding the HDMI breakout which I had already soldered):
![All the stuff I ordered](/files/order.jpg)

I bought a battery powered soldering iron so I can pack it in checked luggage on a flight. It also gives off some jumbo vape pen vibes.

## Soldering

Now to actually solder the HDMI PCB. I looked at [fpga4fun's guide on outputting video over HDMI](https://www.fpga4fun.com/HDMI.html) which suggested that I only needed to worry about the TMDS +/- data and pixel clock +/- lanes. I only soldered those pins to save on jumper cables (ignore the bits of wire, black & white jumpers for now):

![Left side of HDMI breakout board with wires connected to it](/files/hdmi_left.jpg)
![Right side of HDMI breakout board with wires connected to it](/files/hdmi_right.jpg)

## Requirements

According to [DVI 1.0 specification section 2.2.4.1](https://www.fpga4fun.com/files/dvi_spec-V1_0.pdf), the low-pixel format 640x480 @ 60HZ should be supported on all displays, so I settled on that. Plus it's already big enough for 360p video playback.

### Pixel clock

Outputting the low-pixel format actually needs a larger 800x525 frame:
![Sketch of the 800x525 frame composition](/files/frame_diagram.jpg)

The shaded intervals are pulses for vertical and horizontal synchronization. From my understanding, these were originally alotted to give time for the electron beam in a CRT display to reposition itself. They are now repurposed in HDMI for sending other data like audio.

The pixel clock needs to be fast enough to fit 60 of these frames into a single second: `800*525/frame * 60 frames/second / 1,000,000 Hz/Mhz = 25.2 MHz`. It's easier to produce 25MHz since I have an oscillator at 50MHz on my board. This also happens to be the lowest supported HDMI pixel clock.

### Transition Minimized Differential Signaling (TMDS)

There are three TMDS lanes in the HDMI cable. I'm using RGB 24-bit color (4:4:4) so each lane carries 8 bits of color -- 0: red, 1: green, and 2: blue. TMDS itself is pretty impressive. It was designed to minimize electromagnetic interference over copper cabling using a twisted pair of differential signals and an 8-to-10 bit encoding that DC balances the signal. Luckily, Jean P. Nicolle from fpga4fun wrote [a video TMDS encoder](https://www.fpga4fun.com/files/HDMI_test.zip) which I have re-used.

There's 10 bits of data to send per pixel clock on the TMDS lanes so I needed a TMDS clock ten times as fast at 250MHz. Making a clock faster that the 50MHz on-board oscillator was not immediately obvious.

### Phase-locked loop (PLL)

I ran into [PLLs](https://en.wikipedia.org/wiki/Phase-locked_loop) while perusing my board's user manual. I don't fully understand them, but they multiply or divide an input frequency by a constant. Altera has a PLL [IP block](https://en.wikipedia.org/wiki/Semiconductor_intellectual_property_core) which I used to create 25MHz and 250MHz clocks:

![Diagram of the PLL in Quartus](/files/pll.png)

## Implementation in Verilog

I wrote the top-level entity and other components from scratch but re-used Jean's TMDS encoder. The code is still a bit messy so I haven't released it yet. I'll update this post once I do.

## Troubleshooting

As expected, output didn't work on the first try. It took me a couple weeks to go through all the troubleshooting steps.

### Implementation level

I tried running Jean's code exactly as given but it made no difference. I made a test bench and it seemed like the output was right in ModelSim.


### Physical level

I only soldered 8 of the 20 possible pins on the HDMI breakout board so I thought maybe some were missng or misconnected. I checked my pin assignments in Quartus and they match up with the jumpers plugged into the FPGA board and the names on the breakout board. I found a random [Arduino HDMI shield schematic](https://cdn.alchitry.com/emb/hdmi-shield/hdmi-shield.pdf) and saw they had 18/20 pins connected. The DDC SCL/SDA lines seem to be for something unrelated and I'm not worried about supporting hot plug detection for now. I connected all the shield pins to ground and the 5V pin to 5V on my FPGA board, hence all the random bits of wire in my soldered HDMI board from earlier.


### Signal level

I don't have an oscilloscope to check the output signal, so pretty much anything could've be wrong with it.

I completely misunderstood differential signals in my first implementation and was just outputting the negation of the TMDS signals on the negative lanes. Instead, I needed to assign the TMDS IO standard to the output lanes and use a true differential buffer on the signal.

The MAX 10 FPGA I have only supports TMDS as an IO input mode, so it can't even send HDMI compliant signals. This was pretty discouranging but I was already pretty invested in this project. A couple hours of searching through forums led me to [Mike Field's DVI test](http://hamsterworks.co.nz/mediawiki/index.php/Dvid_test) which notes that **specifying the LVDS IO standard instead will still work for some lower resolutions**. MAX 10 supports LVDS output so I set the pin assignments to it.

I couldn't find any good documentation on why TMDS and LVDS might work together here. TMDS is current-mode logic and LVDS is something else. I imagine that there's some magical range of overlap between the two at low speeds. 

A few places discuss converting between the two. I'm going to stick with 640x480 for now so I leave further investigation up to you and/or my future self: https://m.eet.com/media/1135468/330072.pdf, https://www.ti.com/lit/an/scaa059c/scaa059c.pdf, https://www.silabs.com/documents/public/application-notes/AN408.pdf, https://github.com/mattvenn/kicad/tree/master/dvi-pmod, https://electronics.stackexchange.com/questions/130942/transmitting-hdmi-dvi-over-an-fpga-with-no-support-for-tmds, https://hackaday.io/page/5702-dvi-hdmi-pmod-for-an-ice40-fpga.

## Success!

After all this troubleshooting, I finally got something to display:

![FPGA displaying picture on TV](/files/display_image.jpg)

![FPGA displaying a more interesting picture on TV](/files/display_cool_image.jpg)

### Overscan

Many TVs have a feature called overscan that crops the image and stretches it to fit the screen. 

The red border on my monitor below is invisible on my TV:
![Overscan area highlighted in red](/files/display_overscan.jpg)

Historically, this was because old CRT TVs had unreliable image scaling and positioning, so TV stations designated a safe display area. Modern TVs don't need this compensation anymore but still enable this by default because some TV stations will send junk data in these regions. Unfortunately, Samsung has dropped the ball on this: **their TV firmware does not allow customers to disable overscan on 4:3 resolutions**.

## Future work

There's a lot of extensions to this project worth pursuing:

### Text output

Being able to display text makes for easier debugging so I'm implementing [VGA-compatible text mode](https://en.wikipedia.org/wiki/VGA-compatible_text_mode):
![Demo of VGA-compatible text mode with rows of characters from 0 to M](/files/display_text.jpg)

### Audio

HDMI also lets you send audio when video isn't being output. It has something to do with "data island" packets but since I'm not a current adopter of HDMI I can't read the latest specification. Jumping down a few internet rabbit holes should yield some results.

There are also some cool projects that directly produce audio signals over an audio jack. [Fpga4fun.com has one on PWM with a 1-bit DAC](https://www.fpga4fun.com/PWM_DAC_3.html).

### User input

The easiest interface to connect seems to be PS/2 but I don't have a PS/2 keyboard on hand. Interestingly, some older USB keyboards unofficially support fallback to PS/2 over USB. I can solder a female USB port using the [USB to PS/2 adapter pinout on pinouts.ru](https://pinouts.ru/InputCables/usb_ps2_mouse_pinout.shtml) and try out a few USB keyboards.

I was also browsing Adafruit for other components to get and bought a nifty NXP-9 breakout board. It combines an accelerometer, magnetometer, and gyroscope into one package and I'm thinking I could build a cool gesture-based interface with it.

### Soft CPU

Rust has basic support for some Risc-V ISA extensions (I, M, C). I don't have any experience with this but I started reading the ISA specification and it's looking a bit daunting.
