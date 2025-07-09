.PHONY: \
	c clean clean-ui clean-server clean-build \
	all ui server configure \
	p package \
	u update update-server update-ui \

.DEFAULT_GOAL := all

ROOT_DIR:=$(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
UI_DIR:=$(ROOT_DIR)/ui
SERVER_DIR:=$(ROOT_DIR)/server
BUILD_DIR:=$(ROOT_DIR)/build

c clean: clean-ui clean-server clean-build

clean-build:
	rm -rf $(BUILD_DIR)

clean-ui:
	cd $(UI_DIR) && yarn cache clean

clean-server:
	cd $(SERVER_DIR) && cargo clean

u update: update-ui update-server

update-ui:
	cd $(UI_DIR) && yarn install

update-server:
	cd $(SERVER_DIR) && cargo update

all: ui server configure
	
p package: all clean-build
	mkdir -p $(BUILD_DIR)
	cp $(SERVER_DIR)/Rocket.toml $(BUILD_DIR)/Rocket.toml
	rm -rf $(BUILD_DIR)/ui && cp -rf $(UI_DIR)/dist $(BUILD_DIR)/ui
	cp $(SERVER_DIR)/target/$(TARGET)/release/secretary $(BUILD_DIR)/secretary
ifdef TARGET
	rm -f $(BUILD_DIR)/*.tar.gz && cd $(BUILD_DIR) && tar -czf $(BUILD_DIR)/secretary_$(TARGET).tar.gz *
else
	rm -f $(BUILD_DIR)/*.tar.gz && cd $(BUILD_DIR) && tar -czf $(BUILD_DIR)/secretary.tar.gz *
endif
	
server:
ifdef TARGET
	cd $(SERVER_DIR) && cargo build --release --target $(TARGET) --target-dir $(SERVER_DIR)/target
else
	cd $(SERVER_DIR) && cargo build --release --target-dir $(SERVER_DIR)/target
endif

ui:
	cd $(UI_DIR) && yarn build

configure:
