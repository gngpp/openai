# Copyright (C) 2023 gngpp <gngppz@gmail.com>

include $(TOPDIR)/rules.mk

PKG_NAME:=opengpt

PKG_VERSION:=0.2.3

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

PKG_SOURCE:=opengpt-$(PKG_VERSION)-$(PKG_ARCH)-unknown-linux-$(PKG_LIBC).tar.gz
PKG_SOURCE_URL:=https://github.com/gngpp/opengpt/releases/download/v$(PKG_VERSION)/
PKG_HASH:=skip
STRIP:=false

include $(INCLUDE_DIR)/package.mk

define Package/$(PKG_NAME)
  SECTION:=net
  CATEGORY:=Network
  SUBMENU:=Web Servers/Proxies
  TITLE:=opengpt
  URL:=https://github.com/gngpp/opengpt
endef

define Package/$(PKG_NAME)/description
  Not just an unofficial ChatGPT proxy (bypass Cloudflare 403 Access Denied)
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
	echo "opengpt using precompiled binary."
endef

define Package/opengpt/conffiles
/etc/config/opengpt
endef

define Package/opengpt/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/opengpt $(1)/usr/bin/opengpt
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/opengpt.init $(1)/etc/init.d/opengpt
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/opengpt.config $(1)/etc/config/opengpt
endef

$(eval $(call BuildPackage,$(PKG_NAME)))