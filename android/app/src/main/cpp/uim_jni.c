/* JNI for me.sjun.uim.Engine: one native method per C ABI call (engine/include/uim.h).
 * Strings go to Kotlin as UTF-8 byte arrays (JNI's NewStringUTF wants modified UTF-8). */
#include <jni.h>

#include "uim.h"

#define FN(name) Java_me_sjun_uim_Engine_##name
#define H(h) ((UimHandle *)(intptr_t)(h))

static jbyteArray bytes(JNIEnv *env, const uint8_t *ptr, size_t len) {
  jbyteArray array = (*env)->NewByteArray(env, (jsize)len);
  if (array && len && ptr) (*env)->SetByteArrayRegion(env, array, 0, (jsize)len, (const jbyte *)ptr);
  return array;
}

static void with_bytes(JNIEnv *env, jlong h, jbyteArray data, void (*f)(UimHandle *, const uint8_t *, size_t)) {
  jsize len = (*env)->GetArrayLength(env, data);
  jbyte *ptr = (*env)->GetByteArrayElements(env, data, NULL);
  if (!ptr) return;
  f(H(h), (const uint8_t *)ptr, (size_t)len);
  (*env)->ReleaseByteArrayElements(env, data, ptr, JNI_ABORT);
}

JNIEXPORT jlong JNICALL FN(nativeNew)(JNIEnv *env, jclass cls) { return (jlong)(intptr_t)uim_new(); }
JNIEXPORT void JNICALL FN(nativeFree)(JNIEnv *env, jclass cls, jlong h) { uim_free(H(h)); }
JNIEXPORT void JNICALL FN(nativeSetWide)(JNIEnv *env, jclass cls, jlong h, jboolean wide) { uim_set_wide(H(h), wide ? 1 : 0); }
JNIEXPORT void JNICALL FN(nativeSetBottom)(JNIEnv *env, jclass cls, jlong h, jint b) { uim_set_bottom(H(h), (uint32_t)b); }
JNIEXPORT void JNICALL FN(nativeLoadWords)(JNIEnv *env, jclass cls, jlong h, jbyteArray d) { with_bytes(env, h, d, uim_load_words); }
JNIEXPORT void JNICALL FN(nativeLoadBigrams)(JNIEnv *env, jclass cls, jlong h, jbyteArray d) { with_bytes(env, h, d, uim_load_bigrams); }
JNIEXPORT void JNICALL FN(nativeLoadAcronyms)(JNIEnv *env, jclass cls, jlong h, jbyteArray d) { with_bytes(env, h, d, uim_load_acronyms); }
JNIEXPORT jlong JNICALL FN(nativeKanjiNew)(JNIEnv *env, jclass cls, jbyteArray data) {
  jsize len = (*env)->GetArrayLength(env, data);
  jbyte *ptr = (*env)->GetByteArrayElements(env, data, NULL);
  if (!ptr) return 0;
  const UimKanji *k = uim_kanji_new((const uint8_t *)ptr, (size_t)len);
  (*env)->ReleaseByteArrayElements(env, data, ptr, JNI_ABORT);
  return (jlong)(intptr_t)k;
}
JNIEXPORT void JNICALL FN(nativeKanjiFree)(JNIEnv *env, jclass cls, jlong k) { uim_kanji_free((const UimKanji *)(intptr_t)k); }
JNIEXPORT void JNICALL FN(nativeSetKanji)(JNIEnv *env, jclass cls, jlong h, jlong k) { uim_set_kanji(H(h), (const UimKanji *)(intptr_t)k); }
JNIEXPORT void JNICALL FN(nativePress)(JNIEnv *env, jclass cls, jlong h, jint row, jint col, jint dir) {
  uim_press(H(h), (uint32_t)row, (uint32_t)col, (uint32_t)dir);
}
JNIEXPORT void JNICALL FN(nativeHold)(JNIEnv *env, jclass cls, jlong h, jboolean on) { uim_hold(H(h), on ? 1 : 0); }
JNIEXPORT void JNICALL FN(nativeDrag)(JNIEnv *env, jclass cls, jlong h, jint steps) { uim_drag(H(h), (int32_t)steps); }
JNIEXPORT void JNICALL FN(nativeCandidate)(JNIEnv *env, jclass cls, jlong h, jint i) { uim_candidate(H(h), (uint32_t)i); }
JNIEXPORT void JNICALL FN(nativeFlush)(JNIEnv *env, jclass cls, jlong h) { uim_flush(H(h)); }
JNIEXPORT jbyteArray JNICALL FN(nativeCommit)(JNIEnv *env, jclass cls, jlong h) { return bytes(env, uim_commit_ptr(H(h)), uim_commit_len(H(h))); }
JNIEXPORT jbyteArray JNICALL FN(nativePreedit)(JNIEnv *env, jclass cls, jlong h) { return bytes(env, uim_preedit_ptr(H(h)), uim_preedit_len(H(h))); }
JNIEXPORT jbyteArray JNICALL FN(nativeLayout)(JNIEnv *env, jclass cls, jlong h) { return bytes(env, uim_layout_ptr(H(h)), uim_layout_len(H(h))); }
JNIEXPORT jint JNICALL FN(nativeAction)(JNIEnv *env, jclass cls, jlong h) { return (jint)uim_action(H(h)); }
JNIEXPORT jint JNICALL FN(nativeActionArg)(JNIEnv *env, jclass cls, jlong h) { return (jint)uim_action_arg(H(h)); }
JNIEXPORT jint JNICALL FN(nativeCandidateCount)(JNIEnv *env, jclass cls, jlong h) { return (jint)uim_candidate_count(H(h)); }
JNIEXPORT jbyteArray JNICALL FN(nativeCandidateAt)(JNIEnv *env, jclass cls, jlong h, jint i) {
  return bytes(env, uim_candidate_ptr(H(h), (uint32_t)i), uim_candidate_len(H(h), (uint32_t)i));
}
