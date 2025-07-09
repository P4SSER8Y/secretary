.PHONY: clean clean-frontend clean-backend clean-build all frontend backend configure package update update-frontend update-backend
.DEFAULT_GOAL := all

ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
BUILD_DIR:=$(ROOT_DIR)/build

clean: clean-frontend clean-backend clean-build

clean-build:
	rm -rf $(BUILD_DIR)

clean-frontend:
	cd $(ROOT_DIR)/ui && yarn cache clean

clean-backend:
	cargo clean

update: update-frontend update-backend

update-frontend:
	cd $(ROOT_DIR)/ui && yarn install

update-backend:
	cargo update

all: frontend backend configure
	
package: all clean-build
	mkdir -p $(BUILD_DIR)
	cp $(ROOT_DIR)/Rocket.toml $(ROOT_DIR)/build/Rocket.toml
	rm -rf $(BUILD_DIR)/ui && cp -rf $(ROOT_DIR)/ui/dist $(BUILD_DIR)/ui
	cp $(ROOT_DIR)/target/$(TARGET)/release/secretary $(ROOT_DIR)/build/secretary
ifdef TARGET
	rm -f $(BUILD_DIR)/*.tar.gz && cd $(BUILD_DIR) && tar -czf $(BUILD_DIR)/secretary_$(TARGET).tar.gz *
else
	rm -f $(BUILD_DIR)/*.tar.gz && cd $(BUILD_DIR) && tar -czf $(BUILD_DIR)/secretary.tar.gz *
endif
	
backend:
ifdef TARGET
	cd $(ROOT_DIR) && cargo build --release --target $(TARGET) --target-dir $(ROOT_DIR)/target
else
	cd $(ROOT_DIR) && cargo build --release --target-dir $(ROOT_DIR)/target
endif

frontend:
	cd $(ROOT_DIR)/ui && yarn build

configure:
