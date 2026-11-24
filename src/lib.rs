//! A minimal landscape clock for Android tablets: NativeActivity + OpenGL ES 3,
//! written in Rust. Palette is base16 Black Metal (Gorgoroth), taken from
//! `~/.config/kitty/theme.conf`.

mod anim;
mod calendar;
mod clock;
mod counter;
mod pager;

#[cfg(target_os = "android")]
mod text;

#[cfg(target_os = "android")]
use std::ffi::c_void;
#[cfg(target_os = "android")]
use std::time::{Duration, Instant};

#[cfg(target_os = "android")]
#[cfg(target_os = "android")]
use android_activity::input::{InputEvent, MotionAction};
#[cfg(target_os = "android")]
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent, WindowManagerFlags};
#[cfg(target_os = "android")]
use glow::HasContext;

// --- palette -----------------------------------------------------------------
//
// #000000 background, #c1c1c1 foreground, #5f8787 and #8c7f70 accents.

#[cfg(target_os = "android")]
const BG: [f32; 3] = [0.0, 0.0, 0.0];
#[cfg(target_os = "android")]
const TIME_COLOR: [f32; 4] = [0.757, 0.757, 0.757, 1.0]; // #c1c1c1
#[cfg(target_os = "android")]
const PERIOD_COLOR: [f32; 4] = [0.373, 0.529, 0.529, 1.0]; // #5f8787
#[cfg(target_os = "android")]
const DATE_COLOR: [f32; 4] = [0.549, 0.498, 0.439, 1.0]; // #8c7f70
#[cfg(target_os = "android")]
const DAY_COLOR: [f32; 4] = [0.6, 0.6, 0.6, 1.0]; // #999999

#[cfg(target_os = "android")]
const WEEKDAY_INITIALS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

#[cfg(target_os = "android")]
const FONT: &[u8] = include_bytes!("../assets/fonts/Iosevka-Light.ttf");

#[cfg(target_os = "android")]
#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: i32, tag: *const u8, text: *const u8) -> i32;
}

#[cfg(target_os = "android")]
pub(crate) fn android_log(msg: &str) {
    const INFO: i32 = 4;
    let tag = b"clck\0";
    let mut text = msg.as_bytes().to_vec();
    text.push(0);
    unsafe {
        __android_log_write(INFO, tag.as_ptr(), text.as_ptr());
    }
}

// --- EGL ---------------------------------------------------------------------

#[cfg(target_os = "android")]
type EglHandle = *mut c_void;

#[cfg(target_os = "android")]
const EGL_DEFAULT_DISPLAY: EglHandle = std::ptr::null_mut();
#[cfg(target_os = "android")]
const EGL_NO_SURFACE: EglHandle = std::ptr::null_mut();
#[cfg(target_os = "android")]
const EGL_NO_CONTEXT: EglHandle = std::ptr::null_mut();
#[cfg(target_os = "android")]
const EGL_TRUE: u32 = 1;

#[cfg(target_os = "android")]
const EGL_NONE: i32 = 0x3038;
#[cfg(target_os = "android")]
const EGL_RED_SIZE: i32 = 0x3024;
#[cfg(target_os = "android")]
const EGL_GREEN_SIZE: i32 = 0x3023;
#[cfg(target_os = "android")]
const EGL_BLUE_SIZE: i32 = 0x3022;
#[cfg(target_os = "android")]
const EGL_ALPHA_SIZE: i32 = 0x3021;
#[cfg(target_os = "android")]
const EGL_DEPTH_SIZE: i32 = 0x3025;
#[cfg(target_os = "android")]
const EGL_STENCIL_SIZE: i32 = 0x3026;
#[cfg(target_os = "android")]
const EGL_SURFACE_TYPE: i32 = 0x3033;
#[cfg(target_os = "android")]
const EGL_WINDOW_BIT: i32 = 0x0004;
#[cfg(target_os = "android")]
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
#[cfg(target_os = "android")]
const EGL_OPENGL_ES3_BIT: i32 = 0x0040;
#[cfg(target_os = "android")]
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
#[cfg(target_os = "android")]
const EGL_WIDTH: i32 = 0x3057;
#[cfg(target_os = "android")]
const EGL_HEIGHT: i32 = 0x3056;

#[cfg(target_os = "android")]
#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: EglHandle) -> EglHandle;
    fn eglInitialize(dpy: EglHandle, major: *mut i32, minor: *mut i32) -> u32;
    fn eglChooseConfig(
        dpy: EglHandle,
        attribs: *const i32,
        configs: *mut EglHandle,
        config_size: i32,
        num_config: *mut i32,
    ) -> u32;
    fn eglCreateWindowSurface(
        dpy: EglHandle,
        config: EglHandle,
        window: EglHandle,
        attribs: *const i32,
    ) -> EglHandle;
    fn eglCreateContext(
        dpy: EglHandle,
        config: EglHandle,
        share: EglHandle,
        attribs: *const i32,
    ) -> EglHandle;
    fn eglMakeCurrent(dpy: EglHandle, draw: EglHandle, read: EglHandle, ctx: EglHandle) -> u32;
    fn eglSwapBuffers(dpy: EglHandle, surface: EglHandle) -> u32;
    fn eglQuerySurface(dpy: EglHandle, surface: EglHandle, attribute: i32, value: *mut i32) -> u32;
    fn eglGetProcAddress(name: *const u8) -> *const c_void;
    fn eglDestroySurface(dpy: EglHandle, surface: EglHandle) -> u32;
    fn eglDestroyContext(dpy: EglHandle, ctx: EglHandle) -> u32;
    fn eglTerminate(dpy: EglHandle) -> u32;
}

#[cfg(target_os = "android")]
struct Surface {
    display: EglHandle,
    surface: EglHandle,
    context: EglHandle,
    gl: glow::Context,
}

#[cfg(target_os = "android")]
impl Surface {
    unsafe fn create(window: EglHandle) -> Option<Self> {
        let display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
        if display.is_null() {
            android_log("eglGetDisplay failed");
            return None;
        }
        let (mut major, mut minor) = (0, 0);
        if eglInitialize(display, &mut major, &mut minor) != EGL_TRUE {
            android_log("eglInitialize failed");
            return None;
        }

        let config_attribs = [
            EGL_RENDERABLE_TYPE,
            EGL_OPENGL_ES3_BIT,
            EGL_SURFACE_TYPE,
            EGL_WINDOW_BIT,
            EGL_RED_SIZE,
            8,
            EGL_GREEN_SIZE,
            8,
            EGL_BLUE_SIZE,
            8,
            EGL_ALPHA_SIZE,
            8,
            EGL_DEPTH_SIZE,
            0,
            EGL_STENCIL_SIZE,
            0,
            EGL_NONE,
        ];
        let mut config: EglHandle = std::ptr::null_mut();
        let mut num_configs = 0;
        eglChooseConfig(
            display,
            config_attribs.as_ptr(),
            &mut config,
            1,
            &mut num_configs,
        );
        if config.is_null() || num_configs < 1 {
            android_log("no matching EGL config");
            eglTerminate(display);
            return None;
        }

        let surface = eglCreateWindowSurface(display, config, window, [EGL_NONE].as_ptr());
        if surface.is_null() {
            android_log("eglCreateWindowSurface failed");
            eglTerminate(display);
            return None;
        }

        let context_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE];
        let context = eglCreateContext(display, config, EGL_NO_CONTEXT, context_attribs.as_ptr());
        if context.is_null() {
            android_log("eglCreateContext failed");
            eglDestroySurface(display, surface);
            eglTerminate(display);
            return None;
        }

        if eglMakeCurrent(display, surface, surface, context) != EGL_TRUE {
            android_log("eglMakeCurrent failed");
            eglDestroyContext(display, context);
            eglDestroySurface(display, surface);
            eglTerminate(display);
            return None;
        }

        let gl = glow::Context::from_loader_function(|name| eglGetProcAddress(name.as_ptr()));

        android_log(&format!(
            "EGL {}.{} ready, GLES {}",
            major,
            minor,
            gl.get_parameter_string(glow::VERSION)
        ));

        Some(Self {
            display,
            surface,
            context,
            gl,
        })
    }

    unsafe fn size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        eglQuerySurface(self.display, self.surface, EGL_WIDTH, &mut w);
        eglQuerySurface(self.display, self.surface, EGL_HEIGHT, &mut h);
        (w, h)
    }

    unsafe fn swap(&self) {
        eglSwapBuffers(self.display, self.surface);
    }
}

#[cfg(target_os = "android")]
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(self.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            eglDestroyContext(self.display, self.context);
            eglDestroySurface(self.display, self.surface);
            eglTerminate(self.display);
        }
    }
}

// --- drawing -----------------------------------------------------------------

#[cfg(target_os = "android")]
fn group_width(tr: &text::TextRenderer, text: &str, px: f32, tracking: f32) -> f32 {
    tr.measure(text, px, tracking)
}

/// Draws a group of characters, rolling each changed one from below to above.
/// Unchanged characters stay perfectly still.
#[cfg(target_os = "android")]
#[allow(clippy::too_many_arguments)]
unsafe fn draw_group(
    tr: &mut text::TextRenderer,
    gl: &glow::Context,
    group: &anim::Group,
    x: f32,
    baseline: f32,
    px: f32,
    tracking: f32,
    color: [f32; 4],
    clip: [f32; 4],
    now: Instant,
) {
    let current: Vec<char> = group.text.chars().collect();
    let distance = px * anim::DISTANCE_EM;

    match &group.slide {
        None => {
            tr.draw(gl, &group.text, x, baseline, px, color, tracking, clip);
        }
        Some(slide) => {
            let previous: Vec<char> = slide.prev.chars().collect();
            // When the length changes (9 -> 10), right-align the outgoing text so
            // the trailing digits keep their slot.
            let prev_x = if previous.len() == current.len() {
                x
            } else {
                x + group_width(tr, &group.text, px, tracking)
                    - group_width(tr, &slide.prev, px, tracking)
            };

            // Outgoing characters, travelling up and out of the clip band.
            let mut pen = prev_x;
            for (i, ch) in previous.iter().enumerate() {
                let unchanged = previous.len() == current.len()
                    && current.get(i) == Some(ch)
                    && i < current.len();
                let progress = group.progress(i, now);
                if !unchanged && progress < 1.0 {
                    // Same eased offset as the incoming glyph, on the other side,
                    // so both halves of the roll stay exactly one slot apart.
                    let travel = anim::ease_out_back(progress);
                    tr.draw_slot(gl, *ch, pen, baseline - travel * distance, px, color, clip);
                }
                pen += tr.advance(*ch, px) + tracking;
            }

            // Incoming characters, rising into their slot with a small overshoot.
            let mut pen = x;
            for (i, ch) in current.iter().enumerate() {
                let unchanged = previous.get(i) == Some(ch) && previous.len() == current.len();
                let progress = group.progress(i, now);
                let dy = if unchanged || progress >= 1.0 {
                    0.0
                } else {
                    (1.0 - anim::ease_out_back(progress)) * distance
                };
                tr.draw_slot(gl, *ch, pen, baseline + dy, px, color, clip);
                pen += tr.advance(*ch, px) + tracking;
            }
        }
    }
}

#[cfg(target_os = "android")]
unsafe fn draw_centered(
    text: &mut text::TextRenderer,
    gl: &glow::Context,
    label: &str,
    center_x: f32,
    baseline: f32,
    px: f32,
    tracking: f32,
    color: [f32; 4],
    clip: [f32; 4],
) {
    let width = text.measure(label, px, tracking);
    text.draw(
        gl,
        label,
        center_x - width * 0.5,
        baseline,
        px,
        color,
        tracking,
        clip,
    );
}

#[cfg(target_os = "android")]
unsafe fn draw_time_page(
    text: &mut text::TextRenderer,
    gl: &glow::Context,
    anim: &anim::ClockAnim,
    wf: f32,
    hf: f32,
    dx: f32,
    now: Instant,
) {
    // Time, sized to the shorter edge of the landscape window.
    let px = (hf * 0.44).min(wf * 0.26);
    let tracking = px * 0.02;
    let period_px = px * 0.19;
    let period_tracking = period_px * 0.07;
    let gap = px * 0.075;

    let hour_w = group_width(text, &anim.hour.text, px, tracking);
    let minute_w = group_width(text, &anim.minute.text, px, tracking);
    let period_w = group_width(text, &anim.period.text, period_px, period_tracking);
    let colon_w = text.advance(':', px);

    // When the hour gains a digit the whole line has to re-centre; ease that so
    // the minutes glide instead of jumping sideways.
    let hour_w = match &anim.hour.slide {
        Some(slide) => {
            let prev_w = group_width(text, &slide.prev, px, tracking);
            let settle = anim::ease_out_cubic(anim.hour.progress(0, now));
            anim::lerp(prev_w, hour_w, settle)
        }
        None => hour_w,
    };

    let line_w = hour_w + tracking + colon_w + tracking + minute_w;
    let total_w = line_w + gap + period_w;
    let left = (wf - total_w) * 0.5 + dx;

    let baseline = hf * 0.47 + text.centre_offset('0', px);

    // The clip band hides the travelling glyphs until they slide into the line.
    let time_clip = [0.0, baseline - px * 1.02, wf, baseline + px * 0.13];
    let period_clip = [
        0.0,
        baseline - period_px * 1.2,
        wf,
        baseline + period_px * 0.3,
    ];

    let hour_x = left;
    let colon_x = left + hour_w + tracking;
    let minute_x = colon_x + colon_w + tracking;
    let period_x = left + line_w + gap;

    draw_group(
        text, gl, &anim.hour, hour_x, baseline, px, tracking, TIME_COLOR, time_clip, now,
    );
    text.draw(gl, ":", colon_x, baseline, px, TIME_COLOR, 0.0, time_clip);
    draw_group(
        text,
        gl,
        &anim.minute,
        minute_x,
        baseline,
        px,
        tracking,
        TIME_COLOR,
        time_clip,
        now,
    );
    draw_group(
        text,
        gl,
        &anim.period,
        period_x,
        baseline,
        period_px,
        period_tracking,
        PERIOD_COLOR,
        period_clip,
        now,
    );

    // Date just under the time, rolling the same way when the day changes.
    let date_px = (hf * 0.052).clamp(13.0, 48.0);
    let date_tracking = date_px * 0.16;
    let date_w = text.measure(&anim.date.text, date_px, date_tracking);
    let date_baseline = baseline + px * 0.3 + text.centre_offset('0', date_px);
    let date_clip = [
        0.0,
        date_baseline - date_px * 1.05,
        wf,
        date_baseline + date_px * 0.25,
    ];
    draw_group(
        text,
        gl,
        &anim.date,
        (wf - date_w) * 0.5 + dx,
        date_baseline,
        date_px,
        date_tracking,
        DATE_COLOR,
        date_clip,
        now,
    );
}

/// Minimal month grid: month/year header, weekday initials, then the days.
#[cfg(target_os = "android")]
#[cfg(target_os = "android")]
#[derive(Default, Clone, Copy)]
struct CalendarButtons {
    prev: [f32; 4],
    next: [f32; 4],
}

#[cfg(target_os = "android")]
fn hit(rect: [f32; 4], x: f32, y: f32) -> bool {
    x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]
}

/// Month title, weekday initials and day grid, slid horizontally by `dx`.
#[cfg(target_os = "android")]
unsafe fn draw_month(
    text: &mut text::TextRenderer,
    gl: &glow::Context,
    view: &clock::MonthView,
    wf: f32,
    hf: f32,
    dx: f32,
) {
    let full = [0.0, 0.0, wf, hf];

    let title_px = (hf * 0.085).min(wf * 0.06);
    let title_tracking = title_px * 0.14;
    let title_baseline = hf * 0.145 + text.centre_offset('0', title_px);
    draw_centered(
        text,
        gl,
        &view.title,
        wf * 0.5 + dx,
        title_baseline,
        title_px,
        title_tracking,
        TIME_COLOR,
        full,
    );

    let cell_w = wf * 0.78 / 7.0;
    let row_h = hf * 0.55 / 6.0;
    let day_px = (cell_w * 0.5).min(row_h * 0.72).min(hf * 0.09);
    let grid_w = cell_w * 7.0;
    let x0 = (wf - grid_w) * 0.5 + dx;

    let head_px = day_px * 0.62;
    let head_baseline = hf * 0.265 + text.centre_offset('0', head_px);
    for (col, label) in WEEKDAY_INITIALS.iter().enumerate() {
        draw_centered(
            text,
            gl,
            label,
            x0 + cell_w * (col as f32 + 0.5),
            head_baseline,
            head_px,
            0.0,
            DATE_COLOR,
            full,
        );
    }

    let grid_top = hf * 0.32;
    for row in 0..view.weeks {
        for col in 0..7 {
            let Some(day) = view.cells[row * 7 + col] else {
                continue;
            };
            let center_x = x0 + cell_w * (col as f32 + 0.5);
            let center_y = grid_top + row_h * (row as f32 + 0.5);
            let baseline = center_y + text.centre_offset('0', day_px);
            let color = if view.today == Some(day) {
                PERIOD_COLOR
            } else {
                DAY_COLOR
            };
            draw_centered(
                text,
                gl,
                &day.to_string(),
                center_x,
                baseline,
                day_px,
                0.0,
                color,
                full,
            );
        }
    }
}

#[cfg(target_os = "android")]
unsafe fn draw_calendar_page(
    text: &mut text::TextRenderer,
    gl: &glow::Context,
    calendar: &calendar::Calendar,
    wf: f32,
    hf: f32,
    dx: f32,
    now: Instant,
    buttons: &mut CalendarButtons,
) {
    let full = [0.0, 0.0, wf, hf];

    // Outgoing and incoming months slide side by side.
    match calendar.slide(now) {
        Some((progress, previous, dir)) => {
            let eased = anim::ease_out_cubic(progress);
            draw_month(text, gl, previous, wf, hf, dx - dir * eased * wf);
            draw_month(
                text,
                gl,
                calendar.view(),
                wf,
                hf,
                dx + dir * (1.0 - eased) * wf,
            );
        }
        None => draw_month(text, gl, calendar.view(), wf, hf, dx),
    }

    // Month controls stay put while the month itself slides.
    let title_px = (hf * 0.085).min(wf * 0.06);
    let arrow_px = title_px * 0.8;
    let title_baseline = hf * 0.145 + text.centre_offset('0', title_px);
    let arrow_baseline = title_baseline - title_px * 0.02;
    let centers = [wf * 0.155 + dx, wf * 0.845 + dx];
    // Fade the arrows while a month is sliding so they do not sit on top of it.
    let mut arrow_color = DAY_COLOR;
    arrow_color[3] = match calendar.slide(now) {
        Some((progress, _, _)) => 0.15 + 0.85 * anim::ease_out_cubic(progress),
        None => 1.0,
    };
    for (label, center) in [("<", centers[0]), (">", centers[1])] {
        let width = text.measure(label, arrow_px, 0.0);
        text.draw(
            gl,
            label,
            center - width * 0.5,
            arrow_baseline,
            arrow_px,
            arrow_color,
            0.0,
            full,
        );
    }
    let half = (arrow_px * 1.15).max(60.0);
    let mid_y = arrow_baseline - arrow_px * 0.35;
    buttons.prev = [
        centers[0] - half,
        mid_y - half,
        centers[0] + half,
        mid_y + half,
    ];
    buttons.next = [
        centers[1] - half,
        mid_y - half,
        centers[1] + half,
        mid_y + half,
    ];
}

/// Tap counter: the count, centred, in the clock's colour.
#[cfg(target_os = "android")]
unsafe fn draw_counter_page(
    text: &mut text::TextRenderer,
    gl: &glow::Context,
    counter: &counter::Counter,
    wf: f32,
    hf: f32,
    dx: f32,
) {
    let px = (hf * 0.44).min(wf * 0.26);
    let tracking = px * 0.02;
    let baseline = hf * 0.47 + text.centre_offset('0', px);
    let full = [0.0, 0.0, wf, hf];
    draw_centered(
        text,
        gl,
        &counter.count().to_string(),
        wf * 0.5 + dx,
        baseline,
        px,
        tracking,
        TIME_COLOR,
        full,
    );
}

#[cfg(target_os = "android")]
unsafe fn draw(
    surface: &Surface,
    text: &mut text::TextRenderer,
    calendar: &calendar::Calendar,
    anim: &anim::ClockAnim,
    counter: &counter::Counter,
    pos: f32,
    now: Instant,
    buttons: &mut CalendarButtons,
) -> bool {
    let (w, h) = surface.size();
    if w <= 0 || h <= 0 {
        return false;
    }
    let (wf, hf) = (w as f32, h as f32);

    text.begin(&surface.gl, w, h, BG);
    draw_counter_page(
        text,
        &surface.gl,
        counter,
        wf,
        hf,
        pager::page_offset(-1.0, pos, wf),
    );
    draw_time_page(
        text,
        &surface.gl,
        anim,
        wf,
        hf,
        pager::page_offset(0.0, pos, wf),
        now,
    );
    draw_calendar_page(
        text,
        &surface.gl,
        calendar,
        wf,
        hf,
        pager::page_offset(1.0, pos, wf),
        now,
        buttons,
    );
    text.end(&surface.gl);
    surface.swap();
    true
}

// --- immersive mode ----------------------------------------------------------

/// Hides the status and navigation bars. `FULLSCREEN` only covers the status
/// bar, so the tablet's navigation buttons have to be hidden through the
/// activity window: `WindowInsetsController` on API 30+, the legacy system UI
/// flags before that. Runs on the Java main thread because it touches views.
#[cfg(target_os = "android")]
fn hide_system_bars(app: &AndroidApp) {
    let task_app = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        let vm = unsafe { jni::JavaVM::from_raw(task_app.vm_as_ptr().cast()) };
        let activity_raw = task_app.activity_as_ptr() as jni::sys::jobject;
        let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
            use jni::{jni_sig, jni_str, objects::JObject, JValue};

            // Unowned global reference; valid while `app` is alive.
            let activity = unsafe { JObject::from_raw(env, activity_raw) };
            let window = env
                .call_method(
                    &activity,
                    jni_str!("getWindow"),
                    jni_sig!("()Landroid/view/Window;"),
                    &[],
                )?
                .l()?;

            let sdk = android_properties::getprop("ro.build.version.sdk")
                .value()
                .and_then(|v| v.trim().parse::<i32>().ok())
                .unwrap_or(26);

            if sdk >= 30 {
                // Let the window extend behind the bars, then hide them with
                // transient bars revealed only by a swipe.
                env.call_method(
                    &window,
                    jni_str!("setDecorFitsSystemWindows"),
                    jni_sig!("(Z)V"),
                    &[JValue::Bool(false)],
                )?;
                let controller = env
                    .call_method(
                        &window,
                        jni_str!("getInsetsController"),
                        jni_sig!("()Landroid/view/WindowInsetsController;"),
                        &[],
                    )?
                    .l()?;
                // WindowInsets.Type.systemBars() = statusBars | navigationBars | captionBar.
                const SYSTEM_BARS: i32 = 0x1 | 0x2 | 0x4;
                const BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE: i32 = 2;
                env.call_method(
                    &controller,
                    jni_str!("hide"),
                    jni_sig!("(I)V"),
                    &[JValue::Int(SYSTEM_BARS)],
                )?;
                env.call_method(
                    &controller,
                    jni_str!("setSystemBarsBehavior"),
                    jni_sig!("(I)V"),
                    &[JValue::Int(BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)],
                )?;
            } else {
                let decor = env
                    .call_method(
                        &window,
                        jni_str!("getDecorView"),
                        jni_sig!("()Landroid/view/View;"),
                        &[],
                    )?
                    .l()?;
                const IMMERSIVE_STICKY: i32 = 0x1000;
                const HIDE_NAVIGATION: i32 = 0x2;
                const FULLSCREEN: i32 = 0x4;
                const LAYOUT_STABLE: i32 = 0x100;
                const LAYOUT_HIDE_NAVIGATION: i32 = 0x200;
                const LAYOUT_FULLSCREEN: i32 = 0x400;
                let flags = IMMERSIVE_STICKY
                    | HIDE_NAVIGATION
                    | FULLSCREEN
                    | LAYOUT_STABLE
                    | LAYOUT_HIDE_NAVIGATION
                    | LAYOUT_FULLSCREEN;
                env.call_method(
                    &decor,
                    jni_str!("setSystemUiVisibility"),
                    jni_sig!("(I)V"),
                    &[JValue::Int(flags)],
                )?;
            }
            Ok(())
        });
        if let Err(err) = result {
            android_log(&format!("immersive mode failed: {err:?}"));
        }
    }));
}

// --- entry point -------------------------------------------------------------

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    android_log("android_main");

    // A clock that is on screen is useless with the screen asleep: this is the
    // native equivalent of the activity window flag FLAG_KEEP_SCREEN_ON.
    let keep_awake = || {
        app.set_window_flags(
            WindowManagerFlags::KEEP_SCREEN_ON | WindowManagerFlags::FULLSCREEN,
            WindowManagerFlags::empty(),
        )
    };
    keep_awake();
    hide_system_bars(&app);

    let tz = clock::android_timezone();

    let mut pager = pager::Pager::new();
    let mut counter = counter::Counter::new();
    let mut surface: Option<Surface> = None;
    let mut text: Option<text::TextRenderer> = None;
    let mut last_stamp: Option<(u32, u32, String)> = None;
    let mut last_offset = i64::MIN;
    let mut clock_anim: Option<anim::ClockAnim> = None;
    let mut calendar: Option<calendar::Calendar> = None;
    let mut buttons = CalendarButtons::default();
    let mut tap_origin: Option<(f32, f32)> = None;
    let mut dirty = true;
    let mut reassert_awake = false;
    let mut quit = false;

    while !quit {
        // Spend frames only while something is rolling.
        let rolling = pager.animating()
            || clock_anim.as_ref().is_some_and(|a| a.any_active())
            || calendar.as_ref().is_some_and(|c| c.animating());
        let poll = if rolling {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(400)
        };

        app.poll_events(Some(poll), |event| {
            match event {
                PollEvent::Main(MainEvent::InitWindow { .. })
                | PollEvent::Main(MainEvent::WindowResized { .. })
                | PollEvent::Main(MainEvent::RedrawNeeded { .. })
                | PollEvent::Main(MainEvent::ContentRectChanged { .. })
                | PollEvent::Main(MainEvent::Resume { .. })
                | PollEvent::Main(MainEvent::GainedFocus) => {
                    dirty = true;
                    reassert_awake = true;
                }
                PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                    // Drop the EGL surface while the native window is still valid.
                    surface = None;
                }
                PollEvent::Main(MainEvent::Destroy) => quit = true,
                _ => {}
            }
        });

        if quit {
            break;
        }

        // Window flags live on the Activity window, which is recreated on some
        // lifecycle transitions, so put them back whenever we are (re)shown.
        if reassert_awake {
            keep_awake();
            hide_system_bars(&app);
            reassert_awake = false;
        }

        let offset = clock::simulated_offset();
        if last_offset != offset {
            last_offset = offset;
            android_log(&format!("clock offset {offset}s"));
        }
        let now = clock::now_in(tz, offset);
        let hour = format!("{:02}", now.hour);
        let minute = format!("{:02}", now.minute);
        let anim = clock_anim
            .get_or_insert_with(|| anim::ClockAnim::new(&hour, &minute, now.period(), &now.date));
        // Watch the date too: a simulated jump can change it without touching
        // the minute.
        let stamp = (now.hour, now.minute, now.date.clone());
        if last_stamp.as_ref() != Some(&stamp) {
            last_stamp = Some(stamp);
            anim.update(&hour, &minute, now.period(), &now.date, Instant::now());
            dirty = true;
        }
        if anim.any_active() {
            dirty = true;
        }

        let cal = calendar.get_or_insert_with(|| calendar::Calendar::new(&now));
        cal.sync(&now);

        // Swipe paging, plus taps on the calendar's month arrows.
        let width = surface
            .as_ref()
            .map_or(0.0, |s| unsafe { s.size().0 as f32 });
        if width > 0.0 {
            if let Ok(mut events) = app.input_events_iter() {
                loop {
                    let mut touched = false;
                    let more = events.next(|event| {
                        let InputEvent::MotionEvent(motion) = event else {
                            return InputStatus::Unhandled;
                        };
                        if motion.pointer_count() == 0 {
                            return InputStatus::Unhandled;
                        }
                        let pointer = motion.pointer_at_index(0);
                        let (x, y) = (pointer.x(), pointer.y());
                        let now = Instant::now();
                        match motion.action() {
                            MotionAction::Down => {
                                tap_origin = Some((x, y));
                                pager.start_drag(x);
                            }
                            MotionAction::Move => pager.drag_to(x, width),
                            MotionAction::Up => {
                                    let was_tap = tap_origin.take().is_some_and(|(sx, sy)| {
                                        (x - sx).abs() < 24.0 && (y - sy).abs() < 24.0 && !pager.was_swipe(width)
                                    });
                                    if was_tap {
                                        if pager.pos() < -0.98 {
                                            counter.increment();
                                        } else if pager.pos() > 0.98 {
                                            if hit(buttons.prev, x, y) {
                                                cal.shift(-1, now);
                                            } else if hit(buttons.next, x, y) {
                                                cal.shift(1, now);
                                            }
                                        }
                                    }
                                    pager.end_drag(x, width, now);
                            }
                            MotionAction::Cancel => {
                                tap_origin = None;
                                pager.cancel_drag(now);
                            }
                            _ => {}
                        }
                        touched = true;
                        InputStatus::Handled
                    });
                    if touched {
                        dirty = true;
                    }
                    if !more {
                        break;
                    }
                }
            }
        }
        if pager.update(Instant::now()) {
            dirty = true;
        }
        if cal.animating() {
            dirty = true;
        }

        if !dirty {
            continue;
        }

        unsafe {
            if surface.is_none() {
                if let Some(window) = app.native_window() {
                    let ptr = window.ptr().as_ptr() as EglHandle;
                    surface = Surface::create(ptr);
                    // The old TextRenderer's GL objects (shaders, textures, VAOs)
                    // belonged to the previous EGL context, which is gone along
                    // with the previous surface. It must be rebuilt against the
                    // new context, not just created once on first run.
                    if let Some(s) = surface.as_ref() {
                        text = Some(text::TextRenderer::new(&s.gl, FONT));
                    }
                }
            }

            if let (Some(s), Some(t)) = (surface.as_ref(), text.as_mut()) {
                if draw(s, t, cal, anim, &counter, pager.pos(), Instant::now(), &mut buttons) {
                    dirty = false;
                    anim.prune(Instant::now());
                    cal.settle(Instant::now());
                }
            }
        }
    }

    android_log("android_main exit");
    drop(surface);
}
