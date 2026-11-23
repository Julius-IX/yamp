cmake_minimum_required(VERSION 3.22)
project(yamp_discord LANGUAGES CXX)

# Only configured when the SDK is in the build, see build.gradle.kts.
find_package(discord_partner_sdk REQUIRED CONFIG)

add_library(yamp_discord SHARED discord_jni.cpp)
set_target_properties(yamp_discord PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED ON)
target_link_libraries(yamp_discord discord_partner_sdk::discord_partner_sdk log)
