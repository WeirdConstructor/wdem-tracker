# Please note, this project has been deprecated and won't be continued. It has been superseded by [HexoSynth - A modular synthesizer in Rust](https://github.com/WeirdConstructor/HexoSynth)</span>

------

WDemTracker - A music tracker which uses wave-sickle for sound generation
=========================================================================

<a href="http://m8geil.de/data/git/wdem-tracker/res/wdemtracker_sc_20190907.png">
<img align="left" width="638" height="527" src="http://m8geil.de/data/git/wdem-tracker/res/wdemtracker_sc_20190907.png">
</a>

This is a crate that provides an editor and tracker backend for use in other
Rust applications. The editor currently uses ggez for graphics and input
handling, but could be rather easily ported to SDL or other graphics libraries.

The Rust port of @logicomacorp Logicoma's WaveSabre synthesizer for 64k demos
which I called [wave-sickle](https://github.com/WeirdConstructor/wave-sickle)
is responsible for sound synthesis and sample playing.

To goal is to include this tracker into a graphics demo engine (or the
other way around) for writing realtime graphics demos with realtime
music/sound generation.

The WLambda scripting language provides means to setup signal flow
graphs and setting up the wave-sickle synthesizer modules.

The main user is currently my wctr-demo-engine project.

# Status

## 2019-09-07

This is not even really released yet. And not even alpha stage,
also the [wave-sickle](https://github.com/WeirdConstructor/wave-sickle)
port is only boiler plate and basic setup code currently.

I am currently very much implementing basic things like the first
synthesizer port and finally sending note triggers to it.

Just recently I added the audio buffers and audio device thread.

# How to run?

Currently it has not been published to crates.io and you have to clone
the git repositories yourself, but from that it should be just a call
to `cargo run --release` to execute it:

    # git clone https://github.com/WeirdConstructor/wlambda.git
    # git clone https://github.com/WeirdConstructor/wctr-signal-ops.git
    # git clone https://github.com/WeirdConstructor/wdem-tracker.git

    # cd wdem-tracker
    # cargo run --release

Then hit F1 for a short command overview.

# Usage

The tracker usage is different from most trackers and it's input interface
is similar to vi(m) in concept. You have different modes to do different
things and you always get back to the default `[Normal]` mode by hitting the
ESC key. *You Quit by hitting the `q` key!*.

For Help hit *F1*.

# WLambda Tracker API

The whole tracker is configured and signal graph is setup by an WLambda script.
The script is read from the `tracker.wl` file in the current working directory.
This is an example of how such a `tracker.wl` script might look like:

    displayln "audio thread setting start!";

    !g_main = audio_call :signal_group "Main";
    audio_call :track_proxy 5 g_main;

    !g_sub = audio_call :signal_group "Sub";
    !os  = audio_call :op :sin "Sin1" g_sub;
    !os2 = audio_call :op :sin "Sin2" g_sub;

    range 1 100 1 {
        !i = _;
        audio_call :op :sin [str:cat "Sin" i] g_sub;
    };

    !g_inst1 = audio_call :signal_group :Inst1;
    audio_call :op :slaughter "Sl1" g_inst1;
    audio_call :op :audio_send "AS1" g_inst1;

    !r = $[:addmul, 0, 1.0, 0.01];
    audio_call :input "AS1" :vol_l r;
    audio_call :input "AS1" :vol_r r;

    audio_call :thread:quit;
    displayln "audio thread setting end!";

Aside from the [WLambda Reference](https://docs.rs/wlambda/latest/wlambda/prelude/index.html#wlambda-reference)
following functions are available:

## The Modulation Signal Graph

There is a huge vector of floats called the _register_, which holds
the current modulation values. The signal graph is executed every
tick and the operators in that graph can read/write/modify
the _register_ and optionally generate an audio signal. The audio signal
of a signal group (see below) can be completely replaced/altered/overwritten
by the operator.

## Global Functions

### audio\_call _audio-thread-function-name_ {_args_}

## Audio Thread Functions

This section holds all the functions available in the
audio thread, where the setup of the signal network happens.
You can call audio thread functions with `audio_call` and `audio_send`.

### _group-id_ = signal\_group _name_

Creates a new signal group, which is loosely coupling multiple
modulator and/or audio operators. Each signal group also has an
associated audio buffer. The signal groups audio is rendered in the
order the groups were created.

### track\_proxy _track-count_ _group-id_

This function creates a proxy to forward track modulation value signals
into the signal graph. The first _track-count_ tracks are mapped
into the first _track-count_ register indexes.

### _output-register-index_ = op _type_ _name-id_ _group-id_

This command generates a new operator called and identified by _name-id_.
The operator is put into the signal group designated by _group-id_.
The return value is the index in the _register_.

There are currently these types available for _type_:

    sin             - A sinus LFO
                      * Available inputs:
                        amp     - Sine wave amplitude
                        phase   - Sine wave phase
                        vert    - Vertical offset of the sine wave
                        freq    - Frequency of the sine

    audio_send      - An operator that sends the audio of the current
                      signal group and adds it to another signal group.
                      * Available inputs:
                        vol_l   - Linear factor for left channel audio signal
                                  before it's added to the destination bus.
                        vol_r   - Linear factor for right channel audio signal
                                  before it's added to the destination bus.

    slaughter       - The slaughter synthesizer of wave-sickle.

### input _name-id_ _input-name_ _register-operator_

This operation sets the input _input-name_ of an operator identified by
_name-id_ to the given _register-operator_. The register operator is
calculating and returning the actual value that is used for the input of the
operator. The available inputs are listed above in the documentation of the
`op` function.

Following _register-operator_ definitions are possible:

    _float_                 - Fixed non changing value.
                              Example: `0.123`
    $[:reg, _reg-idx_]      - Value of register index _reg-idx_.
                              Example: `$[:reg, 1]`
    $[:mix2, _reg-a-idx_, _reg-b-idx_, _x_]
                            - _x_ is between 0.0 and 1.0. If 0.0 then
                              the value of _reg-a-idx_ is taken, if 1.0
                              value of _reg-b-idx_ is taken. Anything
                              inbetween is a linear mix of the two registers.
    $[:add, _reg-idx_, _value_]
                            - Adds _value_ to the value of _reg-idx_.
    $[:mul, _reg-idx_, _value_]
                            - Multiplies value of _reg-idx_ with _value_.
    $[:addmul, _reg-idx_, _add-value_, _factor-value_]
                            - (reg-value + _add-value_) * _factor-value_
    $[:muladd, _reg-idx_, _factor-value_, _add-value_]
                            - (reg-value * _factor-value_) + _add-value_
    $[:lerp, _reg-idx_, _a_, _b_]
                            - Interpolates linearily between _a_ and _b_
                              with x being the value of _reg-idx_.
    $[:sstep, _reg-idx_, _a_, _b_]
                            - Interpolates smoothsteppy between _a_ and _b_
                              with x being the value of _reg-idx_.
    $[:map, _reg-idx_, _from-a_, _from-b_, _to-a_, _to-b_]
                            - Maps the value of _reg-idx_ from the _from-a_/_from-b_ range
                              to the _to-a_/_to-b_ range.

# License

This project is licensed under the GNU General Public License Version 3 or
later.

## Why GPL?

Picking a license for my code bothered me for a long time. I read many
discussions about this topic. Read the license explanations. And discussed
this matter with other developers.

First about _why I write code for free_ at all:

- It's my passion to write computer programs. In my free time I can
write the code I want, when I want and the way I want. I can freely
allocate my time and freely choose the projects I want to work on.
- To help a friend or member of my family.
- To solve a problem I have.

Those are the reasons why I write code for free. Now the reasons
_why I publish the code_, when I could as well keep it to myself:

- So that it may bring value to users and the free software community.
- Show my work as an artist.
- To get into contact with other developers.
- And it's a nice change to put some more polish on my private projects.

Most of those reasons don't yet justify GPL. The main point of the GPL, as far
as I understand: The GPL makes sure the software stays free software until
eternity. That the user of the software always stays in control. That the users
have _at least the means_ to adapt the software to new platforms or use cases.
Even if the original authors don't maintain the software anymore.
It ultimately prevents _"vendor lock in"_. I really dislike vendor lock in,
especially as developer. Especially as developer I want and need to stay
in control of the computers I use.

Another point is, that my work has a value. If I give away my work without
_any_ strings attached, I effectively work for free. Work for free for
companies. I would compromise the price I can demand for my skill, workforce
and time.

This makes two reasons for me to choose the GPL:

1. I do not want to support vendor lock in scenarios. At least not for free.
   I want to prevent those when I have a choice.
   And before you ask, yes I work for a company that sells closed source
   software. I am not happy about the closed source fact.
   But it pays my bills and gives me the freedom to write free software
   in my free time.
2. I don't want to low ball my own wage and prices by giving away free software
   with no strings attached (for companies).

## If you need a permissive or private license (MIT)

Please contact me if you need a different license and really want to use
my code. As long as I am the only author, I can change the license.
We might find an agreement.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in WLambda by you, shall be licensed as GPLv3 or later,
without any additional terms or conditions.

# Authors

* Weird Constructor <weirdconstructor@gmail.com>
  (You may find me as `WeirdConstructor` on the Rust Discord.)
