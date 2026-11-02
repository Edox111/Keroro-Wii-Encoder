FFmpeg 64-bit static Windows build from www.gyan.dev

Version: 2026-10-25-git-46f23be8b1-full_build-www.gyan.dev

License: GPL v3

Source Code: https://github.com/FFmpeg/FFmpeg/commit/46f23be8b1

External Assets
frei0r plugins:   https://www.gyan.dev/ffmpeg/builds/ffmpeg-frei0r-plugins
lensfun database: https://www.gyan.dev/ffmpeg/builds/ffmpeg-lensfun-db
whisper models:   https://huggingface.co/ggerganov/whisper.cpp/tree/main

git-full build configuration: 

ARCH                      x86 (generic)
big-endian                no
runtime cpu detection     yes
standalone assembly       yes
x86 assembler             nasm
MMX enabled               yes
MMXEXT enabled            yes
SSE enabled               yes
SSSE3 enabled             yes
AESNI enabled             yes
CLMUL enabled             yes
AVX enabled               yes
AVX2 enabled              yes
AVX-512 enabled           yes
AVX-512ICL enabled        yes
XOP enabled               yes
FMA3 enabled              yes
FMA4 enabled              yes
i686 features enabled     yes
CMOV is fast              yes
EBX available             yes
6 registers available     yes
7 registers available     yes
debug symbols             yes
strip symbols             yes
optimize for size         no
optimizations             yes
static                    yes
shared                    no
network support           yes
threading support         pthreads
safe bitstream reader     yes
texi2html enabled         no
perl enabled              yes
pod2man enabled           yes
makeinfo enabled          yes
makeinfo supports HTML    yes
experimental features     yes
xmllint enabled           yes

External libraries:
avisynth                libgsm                  libsvtjpegxs
bzlib                   libharfbuzz             libtheora
cairo                   libilbc                 libtwolame
chromaprint             libjxl                  libuavs3d
frei0r                  liblc3                  libvidstab
gmp                     liblensfun              libvmaf
gnutls                  libmodplug              libvo_amrwbenc
iconv                   libmp3lame              libvorbis
ladspa                  libmysofa               libvpx
lcms2                   liboapv                 libvvenc
libaom                  libopencore_amrnb       libwebp
libaribb24              libopencore_amrwb       libx264
libaribcaption          libopenjpeg             libx265
libass                  libopenmpt              libxavs2
libbluray               libopus                 libxevd
libbs2b                 libplacebo              libxeve
libcaca                 libqrencode             libxml2
libcdio                 libquirc                libxvid
libcodec2               librav1e                libzimg
libdav1d                librist                 libzmq
libdavs2                librubberband           libzvbi
libdvdnav               libshine                lzma
libdvdread              libsnappy               mediafoundation
libflite                libsoxr                 openal
libfontconfig           libspeex                sdl2
libfreetype             libsrt                  whisper
libfribidi              libssh                  zlib
libgme                  libsvtav1

External libraries providing hardware acceleration:
amf                     d3d12va                 nvdec
cuda                    dxva2                   nvenc
cuda_llvm               ffnvcodec               opencl
cuvid                   libmfx                  vaapi
d3d11va                 libvpl                  vulkan

Libraries:
avcodec                 avformat                swscale
avdevice                avutil
avfilter                swresample

Programs:
ffmpeg                  ffplay                  ffprobe

Enabled decoders:
aac                     flv                     pcm_u24be
aac_fixed               fmvc                    pcm_u24le
aac_latm                fourxm                  pcm_u32be
aasc                    fraps                   pcm_u32le
ac3                     frwu                    pcm_u8
ac3_fixed               ftr                     pcm_vidc
acelp_kelvin            g2m                     pcx
adpcm_4xm               g723_1                  pdv
adpcm_adx               g728                    pfm
adpcm_afc               g729                    pgm
adpcm_agm               gdv                     pgmyuv
adpcm_aica              gem                     pgssub
adpcm_argo              gif                     pgx
adpcm_circus            gremlin_dpcm            phm
adpcm_ct                gsm                     photocd
adpcm_dtk               gsm_ms                  pictor
adpcm_ea                h261                    pixlet
adpcm_ea_maxis_xa       h263                    pjs
adpcm_ea_r1             h263i                   png
adpcm_ea_r2             h263p                   ppm
adpcm_ea_r3             h264                    prores
adpcm_ea_xas            h264_amf                prores_raw
adpcm_g722              h264_cuvid              prosumer
adpcm_g726              h264_qsv                psd
adpcm_g726le            hap                     ptx
adpcm_ima_acorn         hca                     qcelp
adpcm_ima_alp           hcom                    qdm2
adpcm_ima_amv           hdr                     qdmc
adpcm_ima_apc           hevc                    qdraw
adpcm_ima_apm           hevc_amf                qoa
adpcm_ima_citrix        hevc_cuvid              qoi
adpcm_ima_cunning       hevc_qsv                qpeg
adpcm_ima_dat4          hnm4_video              qtrle
adpcm_ima_dk3           hq_hqa                  r10k
adpcm_ima_dk4           hqx                     r210
adpcm_ima_ea_eacs       huffyuv                 ra_144
adpcm_ima_ea_sead       hymt                    ra_288
adpcm_ima_escape        iac                     ralf
adpcm_ima_hvqm2         idcin                   rasc
adpcm_ima_hvqm4         idf                     rawvideo
adpcm_ima_iss           iff_ilbm                realtext
adpcm_ima_magix         ilbc                    rka
adpcm_ima_moflex        imc                     rl2
adpcm_ima_mtf           imm4                    roq
adpcm_ima_oki           imm5                    roq_dpcm
adpcm_ima_pda           imm6                    rpza
adpcm_ima_qt            indeo2                  rscc
adpcm_ima_rad           indeo3                  rtv1
adpcm_ima_smjpeg        indeo4                  rv10
adpcm_ima_ssi           indeo5                  rv20
adpcm_ima_wav           interplay_acm           rv30
adpcm_ima_ws            interplay_dpcm          rv40
adpcm_ima_xbox          interplay_video         rv60
adpcm_ms                ipu                     rw2
adpcm_mtaf              jacosub                 s302m
adpcm_n64               jpeg2000                sami
adpcm_psx               jpegls                  sanm
adpcm_psxc              jv                      sbc
adpcm_rhetorex          kgv1                    scpr
adpcm_sanyo             kmvc                    screenpresso
adpcm_sbpro_2           lagarith                sdx2_dpcm
adpcm_sbpro_3           lead                    sga
adpcm_sbpro_4           libaom_av1              sgi
adpcm_swf               libaribb24              sgirle
adpcm_thp               libaribcaption          sheervideo
adpcm_thp_le            libcodec2               shorten
adpcm_vima              libdav1d                simbiosis_imx
adpcm_xa                libdavs2                sipr
adpcm_xmd               libgsm                  siren
adpcm_yamaha            libgsm_ms               smackaud
adpcm_zork              libilbc                 smacker
agm                     libjxl                  smc
ahx                     libjxl_anim             smvjpeg
aic                     liblc3                  snow
alac                    libopencore_amrnb       sol_dpcm
alias_pix               libopencore_amrwb       sp5x
als                     libopus                 speedhq
amrnb                   libspeex                speex
amrwb                   libsvtjpegxs            srgc
amv                     libuavs3d               srt
anm                     libvorbis               ssa
ansi                    libvpx_vp8              stl
anull                   libvpx_vp9              subrip
apac                    libxevd                 subviewer
ape                     libzvbi_teletext        subviewer1
apng                    loco                    sunrast
aptx                    lscr                    svq1
aptx_hd                 m101                    svq3
apv                     mace3                   tak
arbc                    mace6                   targa
argo                    magicyuv                targa_y216
ass                     mdec                    tdsc
asv1                    media100                text
asv2                    metasound               theora
atrac1                  microdvd                thp
atrac3                  mimic                   tiertexseqvideo
atrac3al                misc4                   tiff
atrac3p                 mjpeg                   tmv
atrac3pal               mjpeg_cuvid             truehd
atrac9                  mjpeg_qsv               truemotion1
aura                    mjpegb                  truemotion2
aura2                   mlp                     truemotion2rt
av1                     mmvideo                 truespeech
av1_amf                 mobiclip                tscc
av1_cuvid               motionpixels            tscc2
av1_qsv                 movtext                 tta
avrn                    mp1                     twinvq
avrp                    mp1float                txd
avs                     mp2                     ulti
avui                    mp2float                utvideo
bethsoftvid             mp3                     v210
bfi                     mp3adu                  v210x
bink                    mp3adufloat             vb
binkaudio_dct           mp3float                vble
binkaudio_rdft          mp3on4                  vbn
bintext                 mp3on4float             vc1
bitpacked               mpc7                    vc1_cuvid
bmp                     mpc8                    vc1_qsv
bmv_audio               mpeg1_cuvid             vc1image
bmv_video               mpeg1video              vcr1
bonk                    mpeg2_cuvid             vmdaudio
brender_pix             mpeg2_qsv               vmdvideo
c93                     mpeg2video              vmix
cavs                    mpeg4                   vmnc
cbd2_dpcm               mpeg4_cuvid             vnull
ccaption                mpegvideo               vorbis
cdgraphics              mpl2                    vp3
cdtoons                 msa1                    vp4
cdxl                    mscc                    vp5
cfhd                    msmpeg4v1               vp6
cinepak                 msmpeg4v2               vp6a
clearvideo              msmpeg4v3               vp6f
cljr                    msnsiren                vp7
cllc                    msp2                    vp8
comfortnoise            msrle                   vp8_cuvid
cook                    mss1                    vp8_qsv
cpia                    mss2                    vp9
cri                     msvideo1                vp9_amf
cscd                    mszh                    vp9_cuvid
cyuv                    mts2                    vp9_qsv
dca                     mv30                    vplayer
dds                     mvc1                    vqa
derf_dpcm               mvc2                    vqc
dfa                     mvdv                    vvc
dfpwm                   mvha                    vvc_qsv
dirac                   mwsc                    wady_dpcm
dnxhd                   mxpeg                   wavarc
dolby_e                 nellymoser              wavpack
dpx                     notchlc                 wbmp
dsd_lsbf                nuv                     wcmv
dsd_lsbf_planar         on2avc                  webp
dsd_msbf                opus                    webp_anim
dsd_msbf_planar         osq                     webvtt
dsicinaudio             paf_audio               wmalossless
dsicinvideo             paf_video               wmapro
dss_sp                  pam                     wmav1
dst                     pbm                     wmav2
dvaudio                 pcm_alaw                wmavoice
dvbsub                  pcm_bluray              wmv1
dvdsub                  pcm_dvd                 wmv2
dvvideo                 pcm_dvda                wmv3
dxa                     pcm_f16le               wmv3image
dxtory                  pcm_f24le               wnv1
dxv                     pcm_f32be               wrapped_avframe
eac3                    pcm_f32le               ws_snd1
eacmv                   pcm_f64be               xan_dpcm
eamad                   pcm_f64le               xan_wc3
eatgq                   pcm_lxf                 xan_wc4
eatgv                   pcm_mulaw               xbin
eatqi                   pcm_s16be               xbm
eightbps                pcm_s16be_planar        xface
eightsvx_exp            pcm_s16le               xl
eightsvx_fib            pcm_s16le_planar        xma1
escape124               pcm_s24be               xma2
escape130               pcm_s24daud             xpm
evrc                    pcm_s24le               xsub
exr                     pcm_s24le_planar        xwd
fastaudio               pcm_s32be               y41p
ffv1                    pcm_s32le               ylc
ffvhuff                 pcm_s32le_planar        yop
ffwavesynth             pcm_s64be               yuv4
fic                     pcm_s64le               zero12v
fits                    pcm_s8                  zerocodec
flac                    pcm_s8_planar           zlib
flashsv                 pcm_sga                 zmbv
flashsv2                pcm_u16be
flic                    pcm_u16le

Enabled encoders:
a64multi                hevc_amf                pcm_s24le_planar
a64multi5               hevc_d3d12va            pcm_s32be
aac                     hevc_mf                 pcm_s32le
aac_mf                  hevc_nvenc              pcm_s32le_planar
ac3                     hevc_qsv                pcm_s64be
ac3_fixed               hevc_vaapi              pcm_s64le
ac3_mf                  hevc_vulkan             pcm_s8
adpcm_adx               huffyuv                 pcm_s8_planar
adpcm_argo              jpeg2000                pcm_u16be
adpcm_g722              jpegls                  pcm_u16le
adpcm_g726              libaom_av1              pcm_u24be
adpcm_g726le            libcodec2               pcm_u24le
adpcm_ima_alp           libgsm                  pcm_u32be
adpcm_ima_amv           libgsm_ms               pcm_u32le
adpcm_ima_apm           libilbc                 pcm_u8
adpcm_ima_qt            libjxl                  pcm_vidc
adpcm_ima_ssi           libjxl_anim             pcx
adpcm_ima_wav           liblc3                  pdv
adpcm_ima_ws            libmp3lame              pfm
adpcm_ms                liboapv                 pgm
adpcm_swf               libopencore_amrnb       pgmyuv
adpcm_yamaha            libopenjpeg             phm
alac                    libopus                 png
alias_pix               librav1e                ppm
amv                     libshine                prores
anull                   libspeex                prores_aw
apng                    libsvtav1               prores_ks
aptx                    libsvtjpegxs            prores_ks_vulkan
aptx_hd                 libtheora               qoi
apv_vulkan              libtwolame              qtrle
ass                     libvo_amrwbenc          r10k
asv1                    libvorbis               r210
asv2                    libvpx_vp8              ra_144
av1_amf                 libvpx_vp9              rawvideo
av1_d3d12va             libvvenc                roq
av1_mf                  libwebp                 roq_dpcm
av1_nvenc               libwebp_anim            rpza
av1_qsv                 libx264                 rv10
av1_vaapi               libx264rgb              rv20
av1_vulkan              libx265                 s302m
avrp                    libxavs2                sbc
avui                    libxeve                 sgi
bitpacked               libxvid                 smc
bmp                     ljpeg                   snow
cfhd                    magicyuv                speedhq
cinepak                 mjpeg                   srt
cljr                    mjpeg_qsv               ssa
comfortnoise            mjpeg_vaapi             subrip
dca                     mlp                     sunrast
dfpwm                   movtext                 svq1
dnxhd                   mp2                     targa
dpx                     mp2fixed                text
dsd_msbf                mp3_mf                  tiff
dvbsub                  mpeg1video              truehd
dvdsub                  mpeg2_qsv               tta
dvvideo                 mpeg2_vaapi             ttml
dxv                     mpeg2video              utvideo
eac3                    mpeg4                   v210
exr                     msmpeg4v2               vbn
ffv1                    msmpeg4v3               vc2
ffv1_vulkan             msrle                   vnull
ffvhuff                 msvideo1                vorbis
fits                    nellymoser              vp8_vaapi
flac                    opus                    vp9_qsv
flashsv                 pam                     vp9_vaapi
flashsv2                pbm                     wavpack
flv                     pcm_alaw                wbmp
g723_1                  pcm_bluray              webvtt
gif                     pcm_dvd                 wmav1
h261                    pcm_f32be               wmav2
h263                    pcm_f32le               wmv1
h263p                   pcm_f64be               wmv2
h264_amf                pcm_f64le               wrapped_avframe
h264_d3d12va            pcm_mulaw               xbm
h264_mf                 pcm_s16be               xface
h264_nvenc              pcm_s16be_planar        xsub
h264_qsv                pcm_s16le               xwd
h264_vaapi              pcm_s16le_planar        y41p
h264_vulkan             pcm_s24be               yuv4
hap                     pcm_s24daud             zlib
hdr                     pcm_s24le               zmbv

Enabled hwaccels:
apv_vulkan              hevc_nvdec              vc1_nvdec
av1_d3d11va             hevc_nvdec_cuarray      vc1_nvdec_cuarray
av1_d3d11va2            hevc_vaapi              vc1_vaapi
av1_d3d12va             hevc_vulkan             vp8_nvdec
av1_dxva2               mjpeg_nvdec             vp8_nvdec_cuarray
av1_nvdec               mjpeg_vaapi             vp8_vaapi
av1_nvdec_cuarray       mpeg1_nvdec             vp9_d3d11va
av1_vaapi               mpeg1_nvdec_cuarray     vp9_d3d11va2
av1_vulkan              mpeg2_d3d11va           vp9_d3d12va
dpx_vulkan              mpeg2_d3d11va2          vp9_dxva2
ffv1_vulkan             mpeg2_d3d12va           vp9_nvdec
h263_vaapi              mpeg2_dxva2             vp9_nvdec_cuarray
h264_d3d11va            mpeg2_nvdec             vp9_vaapi
h264_d3d11va2           mpeg2_nvdec_cuarray     vp9_vulkan
h264_d3d12va            mpeg2_vaapi             vvc_vaapi
h264_dxva2              mpeg4_nvdec             wmv3_d3d11va
h264_nvdec              mpeg4_nvdec_cuarray     wmv3_d3d11va2
h264_nvdec_cuarray      mpeg4_vaapi             wmv3_d3d12va
h264_vaapi              prores_raw_vulkan       wmv3_dxva2
h264_vulkan             prores_vulkan           wmv3_nvdec
hevc_d3d11va            vc1_d3d11va             wmv3_nvdec_cuarray
hevc_d3d11va2           vc1_d3d11va2            wmv3_vaapi
hevc_d3d12va            vc1_d3d12va
hevc_dxva2              vc1_dxva2

Enabled parsers:
aac                     dvdsub                  mpegaudio
aac_latm                evc                     mpegvideo
ac3                     ffv1                    opus
adx                     flac                    png
ahx                     ftr                     pnm
amr                     g723_1                  prores
apv                     g729                    prores_raw
av1                     gif                     qoi
avs2                    gsm                     rv34
avs3                    h261                    rw2
bmp                     h263                    sbc
cavsvideo               h264                    sipr
cook                    hdr                     tak
cri                     hevc                    vc1
dca                     ipu                     vorbis
dirac                   jpeg2000                vp3
dnxhd                   jpegxl                  vp8
dnxuc                   jpegxs                  vp9
dolby_e                 lcevc                   vvc
dpx                     misc4                   webp
dvaudio                 mjpeg                   xbm
dvbsub                  mlp                     xma
dvd_nav                 mpeg4video              xwd

Enabled demuxers:
aa                      idf                     pcm_f64be
aac                     iff                     pcm_f64le
aax                     ifv                     pcm_mulaw
ac3                     ilbc                    pcm_s16be
ac4                     image2                  pcm_s16le
ace                     image2_alias_pix        pcm_s24be
acm                     image2_brender_pix      pcm_s24le
act                     image2pipe              pcm_s32be
adf                     image_bmp_pipe          pcm_s32le
adp                     image_cri_pipe          pcm_s8
ads                     image_dds_pipe          pcm_u16be
adx                     image_dpx_pipe          pcm_u16le
aea                     image_exr_pipe          pcm_u24be
afc                     image_gem_pipe          pcm_u24le
aiff                    image_gif_pipe          pcm_u32be
aix                     image_hdr_pipe          pcm_u32le
alp                     image_j2k_pipe          pcm_u8
amr                     image_jpeg_pipe         pcm_vidc
amrnb                   image_jpegls_pipe       pdv
amrwb                   image_jpegxl_pipe       pjs
anm                     image_jpegxs_pipe       pmp
apac                    image_pam_pipe          pp_bnk
apc                     image_pbm_pipe          pva
ape                     image_pcx_pipe          pvf
apm                     image_pfm_pipe          qcp
apng                    image_pgm_pipe          qoa
aptx                    image_pgmyuv_pipe       r3d
aptx_hd                 image_pgx_pipe          rawvideo
apv                     image_phm_pipe          rcwt
aqtitle                 image_photocd_pipe      realtext
argo_asf                image_pictor_pipe       redspark
argo_brp                image_png_pipe          rka
argo_cvg                image_ppm_pipe          rl2
asf                     image_psd_pipe          rm
ass                     image_qdraw_pipe        roq
ast                     image_qoi_pipe          rpl
astc                    image_rw2_pipe          rsd
au                      image_sgi_pipe          rso
av1                     image_sunrast_pipe      rtp
avi                     image_svg_pipe          rtsp
avisynth                image_tiff_pipe         s337m
avr                     image_vbn_pipe          sami
avs                     image_webp_pipe         sap
avs2                    image_xbm_pipe          sbc
avs3                    image_xpm_pipe          sbg
bethsoftvid             image_xwd_pipe          scc
bfi                     imf                     scd
bfstm                   ingenient               sdns
bink                    ipmovie                 sdp
binka                   ipu                     sdr2
bintext                 ircam                   sds
bit                     iss                     sdx
bitpacked               iv8                     segafilm
bmv                     ivf                     ser
boa                     ivr                     sga
bonk                    jacosub                 shorten
brstm                   jpeg_mpf                siff
c93                     jpegxl_anim             simbiosis_imx
caf                     jv                      sln
cavsvideo               ktx                     smacker
cdg                     kux                     smjpeg
cdxl                    kvag                    smush
cine                    laf                     sol
codec2                  lc3                     sox
codec2raw               libgme                  spdif
concat                  libmodplug              srt
dash                    libopenmpt              stl
data                    live_flv                str
daud                    lmlm4                   subviewer
dcstr                   loas                    subviewer1
derf                    lrc                     sup
dfa                     luodat                  svag
dfpwm                   lvf                     svs
dhav                    lxf                     swf
dirac                   m4v                     tak
dnxhd                   matroska                tedcaptions
dsf                     mca                     thp
dsicin                  mcc                     threedostr
dss                     mgsts                   tiertexseq
dts                     microdvd                tmv
dtshd                   mjpeg                   truehd
dv                      mjpeg_2000              tta
dvbsub                  mlp                     tty
dvbtxt                  mlv                     txd
dvdvideo                mm                      ty
dxa                     mmf                     usm
ea                      mods                    v210
ea_cdata                moflex                  v210x
eac3                    mov                     vag
epaf                    mp3                     vc1
evc                     mpc                     vc1t
ffmetadata              mpc8                    vividas
filmstrip               mpegps                  vivo
fits                    mpegts                  vmd
flac                    mpegtsraw               vobsub
flic                    mpegvideo               voc
flv                     mpjpeg                  vpk
fourxm                  mpl2                    vplayer
frm                     mpsub                   vqf
fsb                     msf                     vvc
fwse                    msnwc_tcp               w64
g722                    msp                     wady
g723_1                  mtaf                    wav
g726                    mtv                     wavarc
g726le                  musx                    wc3
g728                    mv                      webm_dash_manifest
g729                    mvi                     webp_anim
gdv                     mvr                     webvtt
genh                    mxf                     wsaud
gif                     mxg                     wsd
gsm                     nc                      wsvqa
gxf                     nistsphere              wtv
h261                    nsp                     wv
h263                    nsv                     wve
h264                    nut                     xa
hca                     nuv                     xbin
hcom                    obu                     xmd
hevc                    ogg                     xmv
hls                     oma                     xvag
hnm                     osq                     xwma
hxvs                    paf                     yop
iamf                    pcm_alaw                yuv4mpegpipe
ico                     pcm_f32be
idcin                   pcm_f32le

Enabled muxers:
a64                     h263                    pcm_s16be
ac3                     h264                    pcm_s16le
ac4                     hash                    pcm_s24be
adts                    hds                     pcm_s24le
adx                     hevc                    pcm_s32be
aea                     hls                     pcm_s32le
aiff                    iamf                    pcm_s8
alp                     ico                     pcm_u16be
amr                     ilbc                    pcm_u16le
amv                     image2                  pcm_u24be
apm                     image2pipe              pcm_u24le
apng                    ipod                    pcm_u32be
aptx                    ircam                   pcm_u32le
aptx_hd                 ismv                    pcm_u8
apv                     iterm2                  pcm_vidc
argo_asf                ivf                     pdv
argo_cvg                jacosub                 psp
asf                     jpeg_mpf                rawvideo
asf_stream              ktx                     rcwt
ass                     kvag                    rm
ast                     latm                    roq
astc                    lc3                     rso
au                      lrc                     rtp
avi                     m4v                     rtp_mpegts
avif                    matroska                rtsp
avm2                    matroska_audio          sap
avs2                    mcc                     sbc
avs3                    md5                     scc
bit                     microdvd                segafilm
caf                     mjpeg                   segment
cavsvideo               mkvtimestamp_v2         smjpeg
chromaprint             mlp                     smoothstreaming
codec2                  mmf                     sox
codec2raw               mov                     spdif
crc                     mp2                     spx
dash                    mp3                     srt
data                    mp4                     stream_segment
daud                    mpeg1system             streamhash
dfpwm                   mpeg1vcd                sup
dirac                   mpeg1video              swf
dnxhd                   mpeg2dvd                tee
dts                     mpeg2svcd               tg2
dv                      mpeg2video              tgp
eac3                    mpeg2vob                truehd
evc                     mpegts                  tta
f4v                     mpjpeg                  ttml
ffmetadata              mxf                     uncodedframecrc
fifo                    mxf_d10                 vc1
filmstrip               mxf_opatom              vc1t
fits                    null                    voc
flac                    nut                     vvc
flv                     obu                     w64
framecrc                oga                     wav
framehash               ogg                     webm
framemd5                ogv                     webm_chunk
g722                    oma                     webm_dash_manifest
g723_1                  opus                    webp
g726                    pcm_alaw                webvtt
g726le                  pcm_f32be               whip
gif                     pcm_f32le               wsaud
gsm                     pcm_f64be               wtv
gxf                     pcm_f64le               wv
h261                    pcm_mulaw               yuv4mpegpipe

Enabled protocols:
async                   httpproxy               rtmps
bluray                  https                   rtmpt
cache                   icecast                 rtmpte
concat                  ipfs_gateway            rtmpts
concatf                 ipns_gateway            rtp
crypto                  librist                 shared
data                    libsrt                  srtp
dtls                    libssh                  subfile
fd                      libzmq                  tcp
ffrtmpcrypt             md5                     tee
ffrtmphttp              mmsh                    tls
file                    mmst                    udp
ftp                     pipe                    udplite
gopher                  prompeg                 windows_handle
gophers                 rtmp
http                    rtmpe

Enabled filters:
a3dscope                dedot                   perlin
aap                     deesser                 perms
abench                  deflate                 perspective
abitscope               deflicker               phase
acompressor             deinterlace_d3d12       photosensitivity
acontrast               deinterlace_qsv         pixdesctest
acopy                   deinterlace_vaapi       pixelize
acrossfade              dejudder                pixscope
acrossover              delogo                  pp7
acrusher                denoise_vaapi           premultiply
acue                    deshake                 premultiply_dynamic
addroi                  deshake_opencl          prewitt
adeclick                despill                 prewitt_opencl
adeclip                 detelecine              procamp_vaapi
adecorrelate            dialoguenhance          program_opencl
adelay                  dilation                pseudocolor
adenorm                 dilation_opencl         psnr
aderivative             displace                pullup
adrawgraph              doubleweave             qp
adrc                    drawbox                 qrencode
adynamicequalizer       drawbox_vaapi           qrencodesrc
adynamicsmooth          drawgraph               quirc
aecho                   drawgrid                random
aemphasis               drawtext                readeia608
aeval                   drawvg                  readvitc
aevalsrc                drmeter                 realtime
aexciter                dynaudnorm              remap
afade                   earwax                  remap_opencl
afdelaysrc              ebur128                 removegrain
afftdn                  edgedetect              removelogo
afftfilt                elbg                    repeatfields
afir                    entropy                 replaygain
afireqsrc               epx                     reverse
afirsrc                 eq                      rgbashift
aformat                 equalizer               rgbtestsrc
afreqshift              erosion                 roberts
afwtdn                  erosion_opencl          roberts_opencl
agate                   estdif                  rotate
agraphmonitor           exposure                rubberband
ahistogram              extractplanes           sab
aiir                    extrastereo             scale
aintegral               fade                    scale2ref
ainterleave             feedback                scale_cuda
alatency                fftdnoiz                scale_d3d11
alimiter                fftfilt                 scale_d3d12
allpass                 field                   scale_qsv
allrgb                  fieldhint               scale_vaapi
allyuv                  fieldmatch              scale_vulkan
aloop                   fieldorder              scdet
alphaextract            fillborders             scdet_vulkan
alphamerge              find_rect               scharr
amerge                  firequalizer            scroll
ametadata               flanger                 segment
amf_capture             flip_vulkan             select
amix                    flite                   selectivecolor
amovie                  floodfill               sendcmd
amplify                 format                  separatefields
amultiply               fps                     setdar
anequalizer             framepack               setfield
anlmdn                  framerate               setparams
anlmf                   framestep               setpts
anlms                   frc_amf                 setrange
anoisesrc               freezedetect            setsar
anull                   freezeframes            settb
anullsink               frei0r                  sharpness_vaapi
anullsrc                frei0r_src              shear
apad                    fruc_vulkan             showcqt
aperms                  fspp                    showcwt
aphasemeter             fsync                   showfreqs
aphaser                 gainmap                 showinfo
aphaseshift             gblur                   showpalette
apsnr                   gblur_vulkan            showspatial
apsyclip                geq                     showspectrum
apulsator               gfxcapture              showspectrumpic
arealtime               gradfun                 showvolume
aresample               gradients               showwaves
areverse                graphmonitor            showwavespic
arls                    grayworld               shuffleframes
arnndn                  greyedge                shufflepixels
asdr                    guided                  shuffleplanes
asegment                haas                    sidechaincompress
aselect                 haldclut                sidechaingate
asendcmd                haldclutsrc             sidedata
asetnsamples            hdcd                    sierpinski
asetpts                 headphone               signalstats
asetrate                hflip                   signature
asettb                  hflip_vulkan            silencedetect
ashowinfo               highpass                silenceremove
asidedata               highshelf               sinc
asisdr                  hilbert                 sine
asoftclip               histeq                  siti
aspectralstats          histogram               smartblur
asplit                  hqdn3d                  smptebars
ass                     hqx                     smptehdbars
astats                  hstack                  sobel
astreamselect           hstack_qsv              sobel_opencl
asubboost               hstack_vaapi            sofalizer
asubcut                 hsvhold                 spectrumsynth
asupercut               hsvkey                  speechnorm
asuperpass              hue                     split
asuperstop              huesaturation           spp
atadenoise              hwdownload              sr_amf
atempo                  hwmap                   ssim
atilt                   hwupload                ssim360
atrim                   hwupload_cuda           stereo3d
avectorscope            hysteresis              stereotools
avgblur                 iccdetect               stereowiden
avgblur_opencl          iccgen                  streamselect
avgblur_vulkan          identity                subtitles
avsynctest              idet                    super2xsai
axcorrelate             il                      superequalizer
azmq                    inflate                 surround
backgroundkey           interlace               swaprect
bandpass                interlace_vulkan        swapuv
bandreject              interleave              tblend
bass                    join                    telecine
bbox                    kerndeint               testsrc
bench                   kirsch                  testsrc2
bilateral               ladspa                  thistogram
bilateral_cuda          lagfun                  threshold
biquad                  latency                 thumbnail
bitplanenoise           latticepal              thumbnail_cuda
blackdetect             lenscorrection          tile
blackdetect_vulkan      lensfun                 tiltandshift
blackframe              libplacebo              tiltshelf
blend                   libvmaf                 tinterlace
blend_vulkan            life                    tlut2
blockdetect             limitdiff               tmedian
blurdetect              limiter                 tmidequalizer
bm3d                    loop                    tmix
boxblur                 loudnorm                tonemap
boxblur_opencl          lowpass                 tonemap_opencl
bs2b                    lowshelf                tonemap_vaapi
bwdif                   lumakey                 tpad
bwdif_cuda              lut                     transpose
bwdif_vulkan            lut1d                   transpose_cuda
cas                     lut2                    transpose_opencl
ccrepack                lut3d                   transpose_vaapi
cellauto                lutrgb                  transpose_vulkan
channelmap              lutyuv                  treble
channelsplit            mandelbrot              tremolo
chorus                  maskedclamp             trim
chromaber_vulkan        maskedmax               unpremultiply
chromahold              maskedmerge             unsharp
chromakey               maskedmin               unsharp_opencl
chromakey_cuda          maskedthreshold         untile
chromanr                maskfun                 uspp
chromashift             mcdeint                 v360
ciescope                mcompand                v360_vulkan
codecview               median                  vaguedenoiser
color                   mergeplanes             varblur
color_vulkan            mestimate               vectorscope
colorbalance            mestimate_d3d12         vflip
colorchannelmixer       metadata                vflip_vulkan
colorchart              midequalizer            vfrdet
colorcontrast           minterpolate            vibrance
colorcorrect            mix                     vibrato
colordetect             monochrome              vidstabdetect
colorhold               morpho                  vidstabtransform
colorize                movie                   vif
colorkey                mpdecimate              vignette
colorkey_opencl         mptestsrc               virtualbass
colorlevels             msad                    vmafmotion
colormap                multiply                volume
colormatrix             negate                  volumedetect
colorspace              nlmeans                 vpp_amf
colorspace_cuda         nlmeans_opencl          vpp_qsv
colorspectrum           nlmeans_vulkan          vqe_amf
colortemperature        nnedi                   vstack
compand                 noformat                vstack_qsv
compensationdelay       noise                   vstack_vaapi
concat                  normalize               w3fdif
convolution             null                    waveform
convolution_opencl      nullsink                weave
convolve                nullsrc                 whisper
copy                    openclsrc               xbr
corr                    oscilloscope            xcorrelate
cover_rect              overlay                 xfade
crop                    overlay_cuda            xfade_opencl
cropdetect              overlay_opencl          xfade_vulkan
crossfeed               overlay_qsv             xmedian
crystalizer             overlay_vaapi           xpsnr
cue                     overlay_vulkan          xstack
curves                  owdenoise               xstack_qsv
datascope               pad                     xstack_vaapi
dblur                   pad_cuda                yadif
dcshift                 pad_opencl              yadif_cuda
dctdnoiz                pad_vaapi               yaepblur
ddagrab                 pal100bars              yuvtestsrc
deband                  pal75bars               zmq
deblock                 palettegen              zoneplate
decimate                paletteuse              zoompan
deconvolve              pan                     zscale

Enabled bsfs:
aac_adtstoasc           h264_mp4toannexb        pgs_frame_merge
ahx_to_mp2              h264_redundant_pps      prores_metadata
apv_metadata            hapqa_extract           remove_extradata
av1_frame_merge         hevc_metadata           setts
av1_frame_split         hevc_mp4toannexb        showinfo
av1_metadata            imx_dump_header         side_data
chomp                   jpegapp                 smpte436m_to_eia608
dca_core                lcevc_merge             text2movsub
dovi_rpu                lcevc_metadata          trace_headers
dovi_split              media100_to_mjpegb      trim
dts2pts                 mjpeg2jpeg              truehd_core
dump_extradata          mjpega_dump_header      vp9_metadata
dv_error_marker         mov2textsub             vp9_raw_reorder
eac3_core               mpeg2_metadata          vp9_superframe
eia608_to_smpte436m     mpeg4_unpack_bframes    vp9_superframe_split
evc_frame_merge         noise                   vvc_metadata
extract_extradata       null                    vvc_mp4toannexb
filter_units            opus_metadata
h264_metadata           pcm_rechunk

Enabled indevs:
dshow                   lavfi                   openal
gdigrab                 libcdio                 vfwcap

Enabled outdevs:
caca

git-full external libraries' versions: 

AMF                 v1.5.2-5-g57ee0cf
aom                 v3.15.2-117-g9ace39e02a
aribb24             v1.0.3-5-g5e9be27
avisynth+           v3.7.5-619-gf176fd8d
cairo               1.18.6-4-g74755964e
chromaprint         1.6.2-1
codec2              1.2.0-108-g310777b1
dav1d               1.5.4-39-gb8b2d726
davs2               1.7-1-gb41cf11
dvdnav              7.0.0-18-g8147ccd
dvdread             7.1.1-125-g0f30df5
ffnvcodec           n13.1.15.0-4-gf205307
flite               v2.2-55-g6c9f20d
fontconfig          2.18.3
freetype            VER-2-15-0
frei0r              v1.5-463-g618dde7
fribidi             v1.0.17-7-gd2f3e1f
game-music-emu      0.6.3-249-gf68963b
gnutls              3.8.13-3
gsm                 1.0.24-1
harfbuzz            15.0.0-2-g75d8ac5d
ladspa-sdk          1.17-2
lame                3.100-3
lc3                 v1.1.3-11-gf9c127f
lcms2               2.19.1-1
lensfun             v0.3.2-3430-g68b5a4e7
libaribcaption      v1.0.0-98-g37dde0f
libass              0.17.5-11-gf61db56
libbluray           1.5.1-4-gf33a738
libbs2b             5ca2d59888
libcaca             0.99.beta20-8
libcdio-paranoia    10.2+2.0.2-2
libilbc             v3.0.4-346-g6adb26d4a4
libjxl              v0.12-snapshot-25-ge47d2a2b
libmodplug          0.8.9.0-5
libmysofa           v1.3.1-79-g648eed0
libplacebo          v7.360.0-246-g1532bf6
librist             v0.2.20-2-g47ad551
libssh              0.12.2-1
libvo-amrwbenc      v0.1.3-7-g884dd24
libvpl              v2.17.0-1-g674d015
libwebp             v1.6.0-362-g1844042
libxml2             v2.15.0-134-gc79c9612
ogg                 v1.3.6-7-g06a5e02
openal-soft         4a52288b77
openapv             v1.1.3.0
opencl-headers      v2026.05.29-11-g82f6031
opencl-loader       v2026.05.29-6-g5192c84
opencore-amr        0.1.6-1
openjpeg2           2.5.4-2
openmpt             OpenMPT-1.32.11.00
opus                v1.6.1-85-ga8b2545e
qrencode            v4.0.0-rc3-102-g715e29f
quirc               v1.2-7-g927d680
rav1e               p20250624-8-g31435de
rubberband          v4.0.0-2-ge4296ac
SDL                 release-2.32.0-261-g8acf7cfc0
shine               3.1.1-59-gab5e352
snappy              1.3.1-1
sox                 sox-14.4.2-184-gf3094754
speex               Speex-1.2.1-57-gf461ff4
srt                 v1.5.7-18-g9f23902
SVT-AV1             v2.3.0-1395-gf2c3fb786
SVT-JPEG-XS         v0.9.0-167-gf472020
theora              v1.2.0alpha1-224-g28fd5ec
twolame             0.4.0-3
uavs3d              1.2
vaapi               2.11.0.pre1-281-gf052e984
vidstab             v1.1.0-291-ge2445c4
vmaf                v3.2.1-68-gf712fa75
vorbis              v1.3.7-37-g1b75110b
vpx                 v1.17.0-85-g900b2a52b
vulkan-loader       sdk-0.1.0-4108-g3172c485
vvenc               v1.14.0-305-g0e7508b
whisper             v1.9.5-134-g231638a0
x264                0480cb05fa
x265                4.3-73-g5143a2a0
xavs2               1.4
xevd                v0.8.0
xeve                v0.7.1
xvidcore            1.3.7-5
zeromq              4.3.5-5
zimg                release-3.0.6-253-g67e0603
zvbi                v0.2.46

