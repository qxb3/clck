//! Battery level for the clock page. The parsing is plain Rust so it can be
//! unit tested; the Android query goes through the sticky
//! `ACTION_BATTERY_CHANGED` broadcast over JNI.

/// Battery state as shown in the corner of the clock page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    /// 0..=100.
    pub percent: u8,
    pub charging: bool,
}

const STATUS_CHARGING: i32 = 2;
const STATUS_FULL: i32 = 5;

/// Builds a [`Battery`] from the `level`, `scale` and `status` intent extras.
pub fn from_extras(level: i32, scale: i32, status: i32) -> Option<Battery> {
    if level < 0 || scale <= 0 {
        return None;
    }
    let percent = ((level as f32 / scale as f32) * 100.0).round().clamp(0.0, 100.0) as u8;
    Some(Battery {
        percent,
        charging: status == STATUS_CHARGING || status == STATUS_FULL,
    })
}

/// Reads the current battery state through the activity context.
#[cfg(target_os = "android")]
pub fn query(app: &android_activity::AndroidApp) -> Option<Battery> {
    use jni::{jni_sig, jni_str, objects::JObject, JValue};

    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let activity_raw = app.activity_as_ptr() as jni::sys::jobject;
    let result = vm.attach_current_thread(|env| -> jni::errors::Result<Option<Battery>> {
        // Unowned global reference; valid while `app` is alive.
        let activity = unsafe { JObject::from_raw(env, activity_raw) };

        let action = env.new_string("android.intent.action.BATTERY_CHANGED")?;
        let filter = env.new_object(
            jni_str!("android/content/IntentFilter"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&action)],
        )?;
        let intent = env
            .call_method(
                &activity,
                jni_str!("registerReceiver"),
                jni_sig!(
                    "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;"
                ),
                &[JValue::Object(&JObject::null()), JValue::Object(&filter)],
            )?
            .l()?;
        if intent.is_null() {
            return Ok(None);
        }

        let mut extra = |name: &str, default: i32| -> jni::errors::Result<i32> {
            let key = env.new_string(name)?;
            env.call_method(
                &intent,
                jni_str!("getIntExtra"),
                jni_sig!("(Ljava/lang/String;I)I"),
                &[JValue::Object(&key), JValue::Int(default)],
            )?
            .i()
        };
        let level = extra("level", -1)?;
        let scale = extra("scale", -1)?;
        let status = extra("status", -1)?;
        Ok(from_extras(level, scale, status))
    });
    match result {
        Ok(battery) => battery,
        Err(err) => {
            crate::android_log(&format!("battery query failed: {err:?}"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_is_scaled_to_a_percentage() {
        let b = from_extras(50, 100, 3).unwrap();
        assert_eq!(b.percent, 50);
        assert!(!b.charging);
        assert_eq!(from_extras(127, 255, 3).unwrap().percent, 50);
    }

    #[test]
    fn charging_and_full_count_as_charging() {
        assert!(from_extras(80, 100, STATUS_CHARGING).unwrap().charging);
        assert!(from_extras(100, 100, STATUS_FULL).unwrap().charging);
    }

    #[test]
    fn missing_data_is_none() {
        assert!(from_extras(-1, 100, 2).is_none());
        assert!(from_extras(50, 0, 2).is_none());
    }
}
