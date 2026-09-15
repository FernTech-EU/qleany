#!/usr/bin/env bash
#
# run_tests.sh -- Build Qleany, generate example projects, and run their tests.
#
# Usage:
#   ./run_tests.sh                  Full suite: build qleany, generate Rust & C++/Qt
#                                   examples, build and test them, then clean up.
#   ./run_tests.sh -g|--generate    Only generate the examples (skip build & test
#                                   of the generated code).
#   ./run_tests.sh --rust           Only run the Rust example (generate, build, test).
#   ./run_tests.sh --cpp-qt         Only run the C++/Qt example (generate, build, test).
#   ./run_tests.sh --no-cleanup      Keep build directories after the run.
#   ./run_tests.sh --deep-clean      Also drop tests/rust/target (cold rebuild next run).
#   ./run_tests.sh --install-deps    Install Ubuntu packages first (Qt6, QCoro, Rust).
#
# Options can be combined, for example:
#   ./run_tests.sh --install-deps --generate
#   ./run_tests.sh --rust --generate    Generate only the Rust example (skip build & test)
#   ./run_tests.sh --cpp-qt -g          Generate only the C++/Qt example
#   ./run_tests.sh --rust --no-cleanup  Run Rust example and keep temp/ for inspection
#
# C++/Qt generates into tests/cpp-qt/tested_project/ and runs both functional
# tests (tests/cpp-qt/functional/) and embedded tests (tested_project/tests/).
#
# When neither --rust nor --cpp-qt is given, both examples are processed.
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

# -----------------------------------------------
# Parse arguments
# -----------------------------------------------
INSTALL_DEPS=false
GENERATE_ONLY=false
SEVENTEEN=false
RUN_RUST=false
RUN_CPPQT=false
NO_CLEANUP=false
DEEP_CLEAN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install-deps)
            INSTALL_DEPS=true
            shift
            ;;
        -g|--generate)
            GENERATE_ONLY=true
            shift
            ;;
        --17)
            SEVENTEEN=true
            shift
            ;;
        --rust)
            RUN_RUST=true
            shift
            ;;
        --cpp-qt)
            RUN_CPPQT=true
            shift
            ;;
        --no-cleanup)
            NO_CLEANUP=true
            shift
            ;;
        --deep-clean)
            DEEP_CLEAN=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --install-deps   Install Ubuntu dependencies (Qt6, QCoro, Rust) via apt/rustup"
            echo "  -g, --generate   Only generate the examples (skip build and test)"
            echo "  --rust           Only process the Rust example"
            echo "  --cpp-qt         Only process the C++/Qt example"
            echo "  --no-cleanup     Keep generated temp/ directories after the run"
            echo "  --deep-clean     Also remove tests/rust/target (forces a cold rebuild)"
            echo "  -h, --help       Show this help message"
            echo ""
            echo "When neither --rust nor --cpp-qt is given, both examples are processed."
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run '$0 --help' for usage."
            exit 1
            ;;
    esac
done

# Default: run both when neither flag is set
if ! $RUN_RUST && ! $RUN_CPPQT; then
    RUN_RUST=true
    RUN_CPPQT=true
fi

if $SEVENTEEN; then
    EXAMPLE_DIR="17"
else
    EXAMPLE_DIR="full"
fi

# -----------------------------------------------
# Install dependencies (optional)
# -----------------------------------------------
if $INSTALL_DEPS; then
    echo "=== Installing Ubuntu dependencies ==="

    echo ""
    echo "--- Installing Teksilo/winit build dependencies ---"
    # `winit` pulls `x11-dl`, whose build script runs pkg-config against x11, so
    # these are needed even for a check-only build.
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        pkg-config \
        libx11-dev \
        libxcb1-dev \
        libxkbcommon-dev \
        libwayland-dev

    echo ""
    echo "--- Installing Qt6 and build tools ---"
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        cmake \
        extra-cmake-modules \
        qt6-base-dev \
        qt6-declarative-dev \
        libgl1-mesa-dev

    echo ""
    echo "--- Installing QCoro ---"
    sudo apt-get install -y \
        qcoro-qt6-dev

    echo ""
    echo "--- Installing Rust (via rustup) ---"
    if command -v rustup &>/dev/null; then
        echo "rustup already installed, updating..."
        rustup update
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    fi

    echo ""
    echo "=== Dependencies installed ==="
fi

echo "=== Qleany Test Suite ==="

# -----------------------------------------------
# 1. Root project: check, build, and test
# -----------------------------------------------
echo ""
echo "--- Root: cargo check ---"
cargo check --workspace

echo ""
echo "--- Root: cargo build ---"
cargo build --workspace

if ! $GENERATE_ONLY; then
    echo ""
    echo "--- Root: cargo test ---"
    cargo test --workspace
fi

# -----------------------------------------------
# 2. Rust example
#    - Generate into tests/rust/tested_project/ from the manifest
#    - Build & run the functional tests (tests/rust/functional/)
# -----------------------------------------------
if $RUN_RUST; then
    RUST_MANIFEST="$REPO_ROOT/examples/rust/$EXAMPLE_DIR/qleany.yaml"
    RUST_TEST_PROJECT="$REPO_ROOT/tests/rust/tested_project"

    # Generated package names are derived from the manifest's application_name,
    # so read it rather than hard-coding "full-rust-app".
    rust_pkg() {
        local app
        app=$(grep -m1 '^  application_name:' "$RUST_MANIFEST" | sed 's/.*: *//' | tr -d '\r')
        # PascalCase -> kebab-case, matching heck::AsKebabCase
        app=$(printf '%s' "$app" | sed -E 's/([a-z0-9])([A-Z])/\1-\2/g' | tr '[:upper:]' '[:lower:]')
        printf '%s-%s' "$app" "$1"
    }

    echo ""
    echo "--- Rust: generate into tests/rust/tested_project/ ---"
    mkdir -p "$RUST_TEST_PROJECT"
    cd "$RUST_TEST_PROJECT"
    "$REPO_ROOT/target/debug/qleany" gen -m "$RUST_MANIFEST"

    # Remove the generated workspace Cargo.toml to avoid nested workspace conflict
    # (the outer tests/rust/Cargo.toml is the real workspace root)
    rm -f "$RUST_TEST_PROJECT/Cargo.toml"

    if ! $GENERATE_ONLY; then
        echo ""
        echo "--- Rust: cargo check (backend + every UI scaffold) ---"
        cd "$REPO_ROOT/tests/rust"
        cargo check --workspace

        # The mocks arm is a second, `#[cfg]`-selected implementation of every
        # single and list model. A default build never compiles it, so without
        # this it rots silently until someone tries to preview a UI.
        echo ""
        echo "--- Rust: cargo check --features mocks (Teksilo mock arm) ---"
        cargo check -p "$(rust_pkg teksilo-ui)" --features mocks

        # Two bins with one name is an output-filename collision that fails
        # `cargo build --workspace`. `cargo check` writes hash-named metadata and
        # passes, so ask cargo for the target list instead of paying for a build.
        echo ""
        echo "--- Rust: binary-name collision check ---"
        cargo metadata --no-deps --format-version 1 \
            | python3 -c '
import json, sys, collections
meta = json.load(sys.stdin)
names = [t["name"] for p in meta["packages"] for t in p["targets"] if "bin" in t["kind"]]
dupes = [n for n, c in collections.Counter(names).items() if c > 1]
if dupes:
    sys.exit("colliding binary names: " + ", ".join(sorted(dupes)))
print("bins: " + ", ".join(sorted(names)))
'

        echo ""
        echo "--- Rust: cargo test (functional tests) ---"
        cargo test --workspace
    fi

    cd "$REPO_ROOT"
fi

# -----------------------------------------------
# 3. C++/Qt example
#    - Generate into tests/cpp-qt/tested_project/ from the manifest
#    - Build & run the functional tests (tests/cpp-qt/functional/)
#    - Build & run the embedded tests (tests/cpp-qt/tested_project/tests/)
# -----------------------------------------------
if $RUN_CPPQT; then
    CPPQT_MANIFEST="$REPO_ROOT/examples/cpp-qt/$EXAMPLE_DIR/qleany.yaml"
    CPPQT_TEST_PROJECT="$REPO_ROOT/tests/cpp-qt/tested_project"

    echo ""
    echo "--- C++/Qt: generate into tests/cpp-qt/tested_project/ ---"
    mkdir -p "$CPPQT_TEST_PROJECT"
    cd "$CPPQT_TEST_PROJECT"
    "$REPO_ROOT/target/debug/qleany" gen -m "$CPPQT_MANIFEST"

    if ! $GENERATE_ONLY; then
        echo ""
        echo "--- C++/Qt: cmake configure (functional + embedded tests) ---"
        cd "$REPO_ROOT/tests/cpp-qt"
        mkdir -p build
        cd build

        # Extract the BUILD_TESTS option prefix from the generated CMakeLists.txt
        BUILD_TESTS_OPT=$(grep -oP '\w+(?=_BUILD_TESTS)' "$CPPQT_TEST_PROJECT/CMakeLists.txt" | head -1)
        cmake .. -D"${BUILD_TESTS_OPT}_BUILD_TESTS=ON"

        echo ""
        echo "--- C++/Qt: cmake build ---"
        cmake --build . --target all -j"$(nproc)"

        echo ""
        echo "--- C++/Qt: ctest (all tests) ---"
        ctest --output-on-failure

        echo ""
        echo "--- C++/Qt: offscreen smoke test (QtWidgets UI) ---"
        if [ -x ./tested_project/src/qtwidgets_app/FullCppQtAppDesktop ]; then
            QT_QPA_PLATFORM=offscreen timeout 5 ./tested_project/src/qtwidgets_app/FullCppQtAppDesktop || true
            echo "QtWidgets UI launched and exited (offscreen)"
        else
            echo "QtWidgets executable not found, skipping"
        fi

        echo ""
        echo "--- C++/Qt: offscreen smoke test (QtQuick UI) ---"
        if [ -x ./tested_project/src/qtquick_app/FullCppQtAppApp ]; then
            QT_QPA_PLATFORM=offscreen timeout 5 ./tested_project/src/qtquick_app/FullCppQtAppApp || true
            echo "QtQuick UI launched and exited (offscreen)"
        else
            echo "QtQuick executable not found, skipping"
        fi
    fi

    cd "$REPO_ROOT"
fi

# -----------------------------------------------
# 4. Clean up
# -----------------------------------------------
if ! $NO_CLEANUP; then
    echo ""
    echo "--- Clean up ---"
    if $RUN_RUST; then
        rm -rf tests/rust/tested_project/*
        # `tests/rust/target` is deliberately kept: it now holds the Teksilo and
        # Slint dependency trees, and wiping it makes every local run a cold
        # rebuild. It is gitignored. Use --deep-clean to remove it.
        if $DEEP_CLEAN; then
            rm -rf tests/rust/target
        fi
    fi
    if $RUN_CPPQT; then
        rm -rf tests/cpp-qt/build
        rm -rf tests/cpp-qt/tested_project/*
    fi
fi

echo ""
if $GENERATE_ONLY; then
    echo "=== Generation complete ==="
else
    echo "=== All tests passed ==="
fi
