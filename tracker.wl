p "audio thread!";
!g_main = signal_group "Main";
track_proxy 5 g_main;
!g_sub = signal_group "Sub";
!os  = op :sin "Sin1" g_sub;
!os2 = op :sin "Sin2" g_sub;
range 1 100 1 { !i = _; op :sin [str:cat "Sin" i] g_sub; };
p "audio thread end!";
