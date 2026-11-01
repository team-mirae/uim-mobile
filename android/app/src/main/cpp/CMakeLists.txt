# The JNI wrapper (uim_jni.c) around the engine's C ABI, linked with the engine's static library
# (built by the app's buildEngine task) into libuim.so.
cmake_minimum_required(VERSION 3.22)
project(uim C)

set(ENGINE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../engine)

if(ANDROID_ABI STREQUAL "arm64-v8a")
  set(RUST_TARGET aarch64-linux-android)
elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
  set(RUST_TARGET armv7-linux-androideabi)
elseif(ANDROID_ABI STREQUAL "x86_64")
  set(RUST_TARGET x86_64-linux-android)
else()
  message(FATAL_ERROR "No engine build for ${ANDROID_ABI}")
endif()

add_library(engine STATIC IMPORTED)
set_target_properties(engine PROPERTIES
  IMPORTED_LOCATION ${ENGINE_DIR}/target/${RUST_TARGET}/release/libuim.a)

add_library(uim SHARED uim_jni.c)
target_include_directories(uim PRIVATE ${ENGINE_DIR}/include)
target_link_libraries(uim engine)
target_link_options(uim PRIVATE -Wl,--gc-sections -Wl,--exclude-libs,ALL)
