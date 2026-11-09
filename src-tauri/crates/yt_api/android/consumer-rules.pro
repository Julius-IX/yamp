# The release build shrinks and renames classes with R8. youtubedl-android unpacks its Python
# with commons-compress and reads yt-dlp output with Jackson, and both reach their classes by
# reflection, which R8 cannot see. Without these rules the first use of yt-dlp fails with an
# ExceptionInInitializerError and every stream lookup and engine update after it fails too.
-keep class com.yausername.** { *; }
-keep class com.fasterxml.jackson.** { *; }
-keep class org.apache.commons.compress.** { *; }
-keep class org.apache.commons.io.** { *; }
-keep class org.tukaani.xz.** { *; }
-dontwarn com.fasterxml.jackson.**
-dontwarn org.apache.commons.compress.**
-dontwarn org.apache.commons.io.**
-dontwarn org.tukaani.xz.**
