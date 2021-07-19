declare name        "studiox-switcher";
declare version     "1.0";
declare author      "Franz Heinzmann";
declare license     "BSD";
declare options     "[osc:on]";

import("stdfaust.lib");

merge2 = _,_: ba.parallelMean(2);

silenceDetect(
    analysisWin,
    dBSilenceTh,
    timeSilenceTh,
    xInput
) =
        ba.linear2db(
            an.rms_envelope_t19(
                analysisWin, 
                xInput
            )
        ) 
        < dBSilenceTh <: fi.pole > (timeSilenceTh * ma.SR);

stereoSilenceFallback(
    analysisWin,
    dBSilenceTh,
    timeSilenceTh,
    mainActive,
    xMainL, xMainR, xBackupL, xBackupR
) =
    ba.select2stereo(cond, xMainL, xMainR, xBackupL, xBackupR)
    with {
        cond = 
            ba.if(
                mainActive,
                silenceDetect(
                    analysisWin,
                    dBSilenceTh,
                    timeSilenceTh, 
                    merge2(xMainL, xMainR)
                ),
                1.0
            );
    };

applySilenceFallback(xBackupL, xBackupR, xMainL, xMainR) =
    stereoSilenceFallback(
        .01,
        vslider("threshold[style:knob][unit:dB]", -60, -70, 0, 0.1),
        vslider("timeout[style:knob]", 1.0, 0.1, 60.0, 0.1),
        1.0,
        xMainL, xMainR,
        xBackupL, xBackupR
    );

switcherN(N, xBackupL, xBackupR) = 
    par(n, N, _,_) : selector(N)
    with {
        selector(1) = ba.select2stereo(
            checkbox("1"),
            xBackupL,
            xBackupR,
            _,_
        );
        selector(n) = ba.select2stereo(
            checkbox("%n"),
            selector(n-1),
            _,_
        );
    };
    
fallbackSwitcherN(N, xBackupL, xBackupR) =
    switcherN(N, xBackupL, xBackupR) : _,_ : applySilenceFallback(xBackupL, xBackupR);

N = 3;
process = par(n, N, _,_),_,_ : fallbackSwitcherN(N) : _,_;
