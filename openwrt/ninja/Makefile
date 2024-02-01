# Copyright (C) 2023 gngpp <gngppz@gmail.com>

include $(TOPDIR)/rules.mk

PKG_NAME:=ninja

PKG_VERSION:=0.9.16

PKG_LICENSE:=MIT
PKG_MAINTAINER:=gngpp <gngppz@gmail.com>

PKG_ARCH=$(ARCH)
PKG_LIBC:=musl
ifeq ($(ARCH),arm)
  PKG_LIBC:=musleabi

  ARM_CPU_FEATURES:=$(word 2,$(subst +,$(space),$(call qstrip,$(CONFIG_CPU_TYPE))))
  ifneq ($(filter $(ARM_CPU_FEATURES),vfp vfpv2),)
    PKG_LIBC:=musleabihf
  endif
endif

PKG_SOURCE:=ninja-$(PKG_VERSION)-$(PKG_ARCH)-unknown-linux-$(PKG_LIBC).tar.gz
PKG_SOURCE_URL:=https://github.com/gngpp/ninja/releases/download/v$(PKG_VERSION)/
PKG_HASH:=skip
STRIP:=false

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  SECTION:=net
  CATEGORY:=Network
  SUBMENU:=Web Servers/Proxies
  TITLE:=ninja
  URL:=https://github.com/gngpp/ninja
endef

define Package/$(PKG_NAME)/description
  Reverse engineered ChatGPT proxy (bypass Cloudflare 403 Access Denied)
endef

define Download/sha256sum
	FILE:=$(PKG_SOURCE).sha256
	URL_FILE:=$(FILE)
	URL:=$(PKG_SOURCE_URL)
	HASH:=skip
endef
$(eval $(call Download,sha256sum))

define Build/Prepare
	mv $(DL_DIR)/$(PKG_SOURCE).sha256 .
	cp $(DL_DIR)/$(PKG_SOURCE) .
	shasum -a 256 -c $(PKG_SOURCE).sha256
	rm $(PKG_SOURCE).sha256 $(PKG_SOURCE)

	tar -xvf $(DL_DIR)/$(PKG_SOURCE)
	tar -C $(PKG_BUILD_DIR)/ -zxf $(DL_DIR)/$(PKG_SOURCE)
endef

define Build/Compile
	echo "ninja using precompiled binary."
endef

define Package/ninja/conffiles
/etc/config/ninja
endef

define Package/ninja/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/ninja $(1)/usr/bin/ninja
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/ninja.init $(1)/etc/init.d/ninja
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/ninja.config $(1)/etc/config/ninja
endef

$(eval $(call BuildPackage,$(PKG_NAME)))