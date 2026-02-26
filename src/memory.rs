use x86_64::{
    structures::paging::{
        PageTable, OffsetPageTable, PhysFrame, Size4KiB,
        FrameAllocator, Mapper, Page, PageTableFlags,
    },
    VirtAddr, PhysAddr,
    registers::control::Cr3,
    structures::paging::page_table::{FrameError, PageTableEntry},
};
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use crate::println;

/// Inicializa un nuevo OffsetPageTable.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// Devuelve una referencia mutable a la tabla de nivel 4 activa.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    unsafe { &mut *page_table_ptr }
}

/// Traduce una dirección virtual a física (wrapper).
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();
    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}

// ==========================================================
// FRAME ALLOCATOR VACÍO (para pruebas)
// ==========================================================

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}

// ==========================================================
// FRAME ALLOCATOR BASADO EN MEMORY MAP (para cuando tengas boot_info)
// ==========================================================

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges
            .flat_map(|r| r.step_by(4096));
        frame_addresses
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// ==========================================================
// FUNCIÓN PARA CREAR UN MAPPING DE EJEMPLO (opcional)
// ==========================================================

pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    let map_to_result = unsafe {
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

// ==========================================================
// FUNCIÓN PARA IMPRIMIR LA TABLA DE PÁGINAS (opcional)
// ==========================================================

pub fn print_page_table(physical_memory_offset: VirtAddr) {
    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

    for (i, entry) in level_4_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 Entry {}: {:?}", i, entry);

            if let Ok(frame) = entry.frame() {
                let phys = frame.start_address();
                let virt = physical_memory_offset + phys.as_u64();
                let ptr = virt.as_mut_ptr();
                let l3_table: &PageTable = unsafe { &*ptr };

                for (j, l3_entry) in l3_table.iter().enumerate() {
                    if !l3_entry.is_unused() {
                        println!("  L3 Entry {}: {:?}", j, l3_entry);
                    }
                }
            }
        }
    }
}