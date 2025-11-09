# Simple Makefile for Span

.PHONY: build release docker-build package

build:
	cargo build --release

package:
	mkdir -p dist/bin
	cp target/release/span dist/bin/span
	cp -r deploy dist/
	cp -r dashboard dist/
	cd dist && tar -czf ../span-$(VERSION)-linux-$(ARCH).tar.gz .

release:
	@echo "Use GitHub Actions workflow to build release artifacts"

docker-build:
	docker build -f deploy/dockerfiles/Dockerfile.control-plane -t span/control-plane:dev .
	docker build -f deploy/dockerfiles/Dockerfile.gateway -t span/gateway:dev .
	docker build -f deploy/dockerfiles/Dockerfile.agent -t span/agent:dev .
