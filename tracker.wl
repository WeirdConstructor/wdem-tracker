displayln "audio thread!";
!g_main = audio_call :signal_group "Main";
displayln "audio thread!?" g_main;
audio_call :track_proxy 5 g_main;
displayln "audio thread!?" g_main;
!g_sub = audio_call :signal_group "Sub";
displayln "audio thread!?" g_main;
!os  = audio_call :op :sin "Sin1" g_sub;
displayln "audio thread!?" g_main;
!os2 = audio_call :op :sin "Sin2" g_sub;
displayln "audio thread!?" g_main;
range 1 100 1 { !i = _; audio_call :op :sin [str:cat "Sin" i] g_sub; };

!g_inst1 = audio_call :signal_group :Inst1;
audio_call :op :slaughter "Sl1" g_inst1;
audio_call :op :audio_send "AS1" g_inst1;
!r = $[:addmul, 0, 1.0, 0.01];
audio_call :input "AS1" :vol_l r;
audio_call :input "AS1" :vol_r r;

audio_call :thread:quit;
displayln "audio thread end!";
