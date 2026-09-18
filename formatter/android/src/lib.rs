#[cfg(panic = "abort")]
compile_error!("JNI requires panic=unwind; use --profile formatter-release instead of --release");

use dprint_markdown_ja_formatter_core::Formatter;
use jni::{
  JNIEnv, JavaVM, NativeMethod,
  objects::{JClass, JString},
  sys::{JNI_ERR, JNI_VERSION_1_6, jint, jstring},
};
use std::{
  ffi::c_void,
  panic::{AssertUnwindSafe, catch_unwind},
  ptr,
};

// get_string uses modified UTF-8 decoding, which can replace unpaired UTF-16
// surrogates. Read UTF-16 explicitly instead: reject malformed Java strings.
fn string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> anyhow::Result<String> {
  anyhow::ensure!(!value.is_null(), "string argument must not be null");
  let array = env.call_method(value, "toCharArray", "()[C", &[])?.l()?;
  let array = jni::objects::JCharArray::from(array);
  let mut chars = vec![0; env.get_array_length(&array)? as usize];
  env.get_char_array_region(&array, 0, &mut chars)?;
  Ok(String::from_utf16(&chars)?)
}

extern "system" fn format(
  mut env: JNIEnv<'_>,
  _: JClass<'_>,
  input: JString<'_>,
  width: jint,
  wrap: JString<'_>,
  emphasis: JString<'_>,
  strong: JString<'_>,
) -> jstring {
  let result = catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<jstring> {
    let input = string(&mut env, &input)?;
    let formatter = Formatter::new(
      width,
      &string(&mut env, &wrap)?,
      &string(&mut env, &emphasis)?,
      &string(&mut env, &strong)?,
    )?;
    let output = formatter.format(&input)?;
    Ok(env.new_string(output.as_ref())?.into_raw())
  }));
  // Exception creation is inside a second guard; no Rust unwind may leave FFI.
  let _ = catch_unwind(AssertUnwindSafe(|| {
    let (class, message) = match &result {
      Ok(Ok(_)) => return,
      Ok(Err(error)) => (
        "java/lang/IllegalArgumentException",
        format!("Markdown formatting failed: {error:#}"),
      ),
      Err(_) => ("java/lang/RuntimeException", "Markdown formatter panicked".to_owned()),
    };
    if !env.exception_check().unwrap_or(true) {
      let _ = env.throw_new(class, message);
    }
  }));
  match result {
    Ok(Ok(value)) => value,
    _ => ptr::null_mut(),
  }
}

/// The only exported JNI symbol. The library owns the stable facade package.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
  catch_unwind(AssertUnwindSafe(|| -> jni::errors::Result<jint> {
    let mut env = vm.get_env()?;
    env.register_native_methods(
      "io/warpnine/markdownja/MarkdownFormatter",
      &[NativeMethod {
        name: "formatNative".into(),
        sig: "(Ljava/lang/String;ILjava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".into(),
        fn_ptr: format as *mut c_void,
      }],
    )?;
    Ok(JNI_VERSION_1_6)
  }))
  .ok()
  .and_then(Result::ok)
  .unwrap_or(JNI_ERR)
}
