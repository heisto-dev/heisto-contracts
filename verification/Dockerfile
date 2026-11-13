FROM solanafoundation/solana-verifiable-build@sha256:12fd4c0a0790f0fc41ef74b0cdb6bccc167ba6137eb1adb749bac649481c86bd

# Binary and checksum come from the original official Agave 4.3.2 release.
RUN set -eu; \
    mkdir -p /opt/agave; \
    curl --fail --location --proto '=https' --tlsv1.2 \
      https://github.com/anza-xyz/agave/releases/download/v4.3.2/solana-release-x86_64-unknown-linux-gnu.tar.bz2 \
      --output /tmp/agave.tar.bz2; \
    echo '5c9599d7fa397375df69d58e8d4f550ee45463dd6d1ecad9e5a1f053ce8e0933  /tmp/agave.tar.bz2' | sha256sum --check; \
    tar -xjf /tmp/agave.tar.bz2 -C /opt/agave; \
    echo 'fecfad827f44c0ff788117788bef8812ccdf7ed85c02910558eaa540b8914a03  /opt/agave/solana-release/bin/cargo-build-sbf' | sha256sum --check; \
    rustup toolchain install 1.88.0 --profile minimal; \
    /opt/agave/solana-release/bin/cargo-build-sbf --install-only --tools-version v1.57; \
    rm /tmp/agave.tar.bz2

# The recorded release used Cargo 1.88.0 with the SBF compiler from
# platform-tools v1.57. A newer Cargo changes the output even with this rustc.
ENV RUSTC=/root/.cache/solana/v1.57/platform-tools/rust/bin/rustc
ENV RUSTDOC=/root/.cache/solana/v1.57/platform-tools/rust/bin/rustdoc
ENV PATH=/root/.rustup/toolchains/1.88.0-x86_64-unknown-linux-gnu/bin:/opt/agave/solana-release/bin:${PATH}
