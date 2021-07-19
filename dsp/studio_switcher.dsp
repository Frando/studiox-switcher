declare name        "studiox-switcher";
declare version     "1.0";
declare author      "Franz Heinzmann";
declare license     "BSD";
declare options     "[osc:on]";

import("stdfaust.lib");

merge2 = _,_: ba.parallelMean(2);

silenceDetect(analysisWin, dBSilenceTh, timeSilenceTh, xInput) =
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

switcher(x1L, x1R, x2L, x2R, x3L, x3R, x4L, x4R) =
    ba.select2stereo(checkbox("1on"),
        ba.select2stereo(checkbox("2on"),
            ba.select2stereo(checkbox("3on"), x4L, x4R, x3L, x3R),
            x2L, x2R
        ),
        x1L, x1R
    )
    : _,_ : applySilenceFallback(x4L, x4R);

fallbackSwitcher(x1L, x1R, x2L, x2R, x3L, x3R, x4L, x4R) =
    switcher(x1L, x1R, x2L, x2R, x3L, x3R, x4L, x4R)
    : applySilenceFallback(x4L, x4R);

process = par(i, 4, _,_) : switcher : _,_;
