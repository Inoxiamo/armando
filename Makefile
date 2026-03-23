.PHONY: fmt fmt-check test clippy pre-release package ci-container sonar-local install-local

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

test:
	cargo test --all-targets

clippy:
	cargo clippy --all-targets -- -D warnings

pre-release:
	bash scripts/release/pre-release-check.sh $(TAG)

package:
	bash scripts/release/package-release.sh "$(BINARY)" "$(TARGET)" "$(VERSION)"

ci-container:
	bash scripts/ci/run-container-tests.sh "$(or $(LABEL),ci-local)"

sonar-local:
	bash scripts/dev/run-sonar-local.sh

install-local:
	bash scripts/dev/install-local.sh
