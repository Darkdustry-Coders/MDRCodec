#!/usr/bin/env sh

set -e

export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk}"

cargo build --features lz4

cd plugin
./gradlew build
cd ..

mkdir -p server-test
cd server-test
if [ ! -f server.jar ]; then
    echo "Downloading Mindustry server..."
    curl -L 'https://github.com/Anuken/Mindustry/releases/download/v158.1/server-release.jar' -o server.jar
fi
mkdir -p config/mods
mkdir -p lib
mv ../target/debug/libmdrcodec.so lib || true
mv ../plugin/build/libs/MDRCoderPlugin.jar config/mods || true

libdir="${LD_LIBRARY_PATH:-/usr/lib}"

LD_LIBRARY_PATH="$(realpath lib):$libdir" "$JAVA_HOME/bin/java" \
    -Djava.library.path="$(realpath lib)" \
    --add-modules=jdk.incubator.foreign --enable-native-access=ALL-UNNAMED \
    --add-opens=java.base/java.io=ALL-UNNAMED --add-opens=java.base/java.nio=ALL-UNNAMED \
    -jar server.jar
