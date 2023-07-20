# Copyright (C) 2023 gngpp <gngppz@gmail.com>

include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-opengpt
PKG_VERSION:=1.0.4

PKG_MAINTAINER:=gngpp <gngppz@gmail.com>

LUCI_TITLE:=LuCI support for OpenGPT
LUCI_DEPENDS:=+opengpt
LUCI_PKGARCH:=all

define Package/$(PKG_NAME)/postinst
#!/bin/sh
[ -n "${IPKG_INSTROOT}" ] || {
	( . /etc/uci-defaults/luci-opengpt ) && rm -f /etc/uci-defaults/luci-opengpt
	exit 0
}
endef

include $(TOPDIR)/feeds/luci/luci.mk

# call BuildPackage - OpenWrt buildroot signature