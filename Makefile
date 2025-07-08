.PHONY: clean clean-frontend clean-backend all frontend backend configure package update update-frontend update-backend
.DEFAULT_GOAL := all

ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
BUILD_DIR:=$(ROOT_DIR)/build

ifndef TARGET
	TARGET:=$(shell source $(ROOT_DIR)/scripts/utils.sh && get_target || exit 1)
	ifeq ($(TARGET),)
        $(error Error: get_target failed)
    endif
endif

clean: clean-frontend clean-backend
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
	
package: all
	rm -f $(BUILD_DIR)/*.tar.gz && cd $(BUILD_DIR) && tar -czf $(BUILD_DIR)/secretary_$(TARGET).tar.gz *
	
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)
	
backend: $(BUILD_DIR)
ifndef TARGET
	$(error Target '$@' requires TARGET parameter. Usage: make $@ TARGET=<target>)
endif
	cd $(ROOT_DIR) && cargo build --release --target $(TARGET) --target-dir $(ROOT_DIR)/target
	cp $(ROOT_DIR)/target/$(TARGET)/release/secretary $(ROOT_DIR)/build/secretary

frontend: $(BUILD_DIR)
	cd $(ROOT_DIR)/ui && yarn build && rm -rf $(BUILD_DIR)/ui && cp -rf $(ROOT_DIR)/ui/dist $(BUILD_DIR)/ui

configure: $(BUILD_DIR) $(ROOT_DIR)/Rocket.toml
	cp $(ROOT_DIR)/Rocket.toml $(ROOT_DIR)/build/Rocket.toml
