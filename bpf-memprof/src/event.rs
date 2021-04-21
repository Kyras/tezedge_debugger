// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use core::convert::TryFrom;

#[derive(Clone, Copy)]
pub struct Hex64(pub u64);

#[derive(Clone, Copy)]
pub struct Hex32(pub u32);

pub trait Pod
where
    Self: Sized,
{
    const DISCRIMINANT: Option<u32>;
    const SIZE: usize;

    fn from_slice(s: &[u8]) -> Option<Self>;
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct CommonHeader {
    ty: u16,
    flags: u8,
    preempt_count: u8,
    pid: u32,
}

impl Pod for CommonHeader {
    const DISCRIMINANT: Option<u32> = None;
    const SIZE: usize = 0x08;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(CommonHeader {
            ty: u16::from_ne_bytes(TryFrom::try_from(&s[0x00..0x02]).unwrap()),
            flags: s[0x02],
            preempt_count: s[0x03],
            pid: u32::from_ne_bytes(TryFrom::try_from(&s[0x04..0x08]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct KFree {
    call_site: Hex64,
    pub ptr: Hex64,
}

impl Pod for KFree {
    const DISCRIMINANT: Option<u32> = Some(1);
    const SIZE: usize = 0x10;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(KFree {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct KMAlloc {
    call_site: Hex64,
    pub ptr: Hex64,
    bytes_req: Hex64,
    pub bytes_alloc: Hex64,
    gfp_flags: Hex32,
}

impl Pod for KMAlloc {
    const DISCRIMINANT: Option<u32> = Some(2);
    const SIZE: usize = 0x24;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(KMAlloc {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
            bytes_req: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x10..0x18]).unwrap())),
            bytes_alloc: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x18..0x20]).unwrap())),
            gfp_flags: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x20..0x24]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct KMAllocNode {
    call_site: Hex64,
    pub ptr: Hex64,
    bytes_req: Hex64,
    pub bytes_alloc: Hex64,
    gfp_flags: Hex32,
    node: Hex32,
}

impl Pod for KMAllocNode {
    const DISCRIMINANT: Option<u32> = Some(3);
    const SIZE: usize = 0x28;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(KMAllocNode {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
            bytes_req: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x10..0x18]).unwrap())),
            bytes_alloc: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x18..0x20]).unwrap())),
            gfp_flags: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x20..0x24]).unwrap())),
            node: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x24..0x28]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct CacheAlloc {
    call_site: Hex64,
    pub ptr: Hex64,
    bytes_req: Hex64,
    pub bytes_alloc: Hex64,
    gfp_flags: Hex32,
}

impl Pod for CacheAlloc {
    const DISCRIMINANT: Option<u32> = Some(4);
    const SIZE: usize = 0x24;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(CacheAlloc {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
            bytes_req: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x10..0x18]).unwrap())),
            bytes_alloc: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x18..0x20]).unwrap())),
            gfp_flags: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x20..0x24]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct CacheAllocNode {
    call_site: Hex64,
    pub ptr: Hex64,
    bytes_req: Hex64,
    pub bytes_alloc: Hex64,
    gfp_flags: Hex32,
    node: Hex32,
}

impl Pod for CacheAllocNode {
    const DISCRIMINANT: Option<u32> = Some(5);
    const SIZE: usize = 0x28;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(CacheAllocNode {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
            bytes_req: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x10..0x18]).unwrap())),
            bytes_alloc: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x18..0x20]).unwrap())),
            gfp_flags: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x20..0x24]).unwrap())),
            node: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x24..0x28]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct CacheFree {
    call_site: Hex64,
    pub ptr: Hex64,
}

impl Pod for CacheFree {
    const DISCRIMINANT: Option<u32> = Some(6);
    const SIZE: usize = 0x10;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(CacheFree {
            call_site: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ptr: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageAlloc {
    pfn: Hex64,
    pub order: u32,
    gfp_flags: Hex32,
    migrate_ty: i32,
}

impl Pod for PageAlloc {
    const DISCRIMINANT: Option<u32> = Some(7);
    const SIZE: usize = 0x14;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageAlloc {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            order: u32::from_ne_bytes(TryFrom::try_from(&s[0x08..0x0c]).unwrap()),
            gfp_flags: Hex32(u32::from_ne_bytes(TryFrom::try_from(&s[0x0c..0x10]).unwrap())),
            migrate_ty: i32::from_ne_bytes(TryFrom::try_from(&s[0x10..0x14]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageAllocExtFrag {
    pfn: Hex64,
    pub alloc_order: u32,
    pub fallback_order: u32,
    alloc_migrate_ty: i32,
    fallback_migrate_ty: i32,
    change_ownership: i32,
}

impl Pod for PageAllocExtFrag {
    const DISCRIMINANT: Option<u32> = Some(8);
    const SIZE: usize = 0x1c;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageAllocExtFrag {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            alloc_order: u32::from_ne_bytes(TryFrom::try_from(&s[0x08..0x0c]).unwrap()),
            fallback_order: u32::from_ne_bytes(TryFrom::try_from(&s[0x0c..0x10]).unwrap()),
            alloc_migrate_ty: i32::from_ne_bytes(TryFrom::try_from(&s[0x10..0x14]).unwrap()),
            fallback_migrate_ty: i32::from_ne_bytes(TryFrom::try_from(&s[0x14..0x18]).unwrap()),
            change_ownership: i32::from_ne_bytes(TryFrom::try_from(&s[0x18..0x1c]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageAllocZoneLocked {
    pfn: Hex64,
    pub order: u32,
    migrate_ty: i32,
}

impl Pod for PageAllocZoneLocked {
    const DISCRIMINANT: Option<u32> = Some(9);
    const SIZE: usize = 0x10;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageAllocZoneLocked {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            order: u32::from_ne_bytes(TryFrom::try_from(&s[0x08..0x0c]).unwrap()),
            migrate_ty: i32::from_ne_bytes(TryFrom::try_from(&s[0x0c..0x10]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageFree {
    pfn: Hex64,
    pub order: u32,
}

impl Pod for PageFree {
    const DISCRIMINANT: Option<u32> = Some(10);
    const SIZE: usize = 0x0c;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageFree {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            order: u32::from_ne_bytes(TryFrom::try_from(&s[0x08..0x0c]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageFreeBatched {
    pfn: Hex64,
}

impl Pod for PageFreeBatched {
    const DISCRIMINANT: Option<u32> = Some(11);
    const SIZE: usize = 0x08;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageFreeBatched {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PagePcpuDrain {
    pfn: Hex64,
    pub order: u32,
    migrate_ty: i32,
}

impl Pod for PagePcpuDrain {
    const DISCRIMINANT: Option<u32> = Some(12);
    const SIZE: usize = 0x10;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PagePcpuDrain {
            pfn: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            order: u32::from_ne_bytes(TryFrom::try_from(&s[0x08..0x0c]).unwrap()),
            migrate_ty: i32::from_ne_bytes(TryFrom::try_from(&s[0x0c..0x10]).unwrap()),
        })
    }
}

#[cfg_attr(feature = "client", derive(serde::Serialize))]
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub struct PageFaultUser {
    address: Hex64,
    ip: Hex64,
    error_code: u64,
}

impl Pod for PageFaultUser {
    const DISCRIMINANT: Option<u32> = Some(13);
    const SIZE: usize = 0x18;

    #[inline(always)]
    fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() < Self::SIZE {
            return None;
        }
        Some(PageFaultUser {
            address: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x00..0x08]).unwrap())),
            ip: Hex64(u64::from_ne_bytes(TryFrom::try_from(&s[0x08..0x10]).unwrap())),
            error_code: u64::from_ne_bytes(TryFrom::try_from(&s[0x10..0x18]).unwrap()),
        })
    }
}
