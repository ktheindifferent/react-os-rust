.code16
.section .ap_boot, "ax"
.global ap_trampoline_start
.global ap_trampoline_end

ap_trampoline_start:
    cli
    cld
    
    # Load GDT pointer from boot info structure
    lgdt ap_gdt_ptr
    
    # Enable protected mode
    mov %cr0, %eax
    or $1, %eax
    mov %eax, %cr0
    
    # Far jump to 32-bit code
    ljmp $0x08, $ap_protected_mode
    
.code32
ap_protected_mode:
    # Set up segments
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss
    
    # Enable PAE
    mov %cr4, %eax
    or $0x20, %eax
    mov %eax, %cr4
    
    # Load CR3 with page table
    mov ap_cr3, %eax
    mov %eax, %cr3
    
    # Enable long mode
    mov $0xC0000080, %ecx
    rdmsr
    or $0x100, %eax
    wrmsr
    
    # Enable paging
    mov %cr0, %eax
    or $0x80000000, %eax
    mov %eax, %cr0
    
    # Jump to 64-bit code
    ljmp $0x08, $ap_long_mode
    
.code64
ap_long_mode:
    # Set up 64-bit segments
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss
    
    # Load stack pointer
    mov ap_stack_top, %rsp
    
    # Load IDT
    lidt ap_idt_ptr
    
    # Jump to C entry point
    mov ap_entry_point, %rax
    jmp *%rax

.align 8
ap_boot_info:
ap_cr3:         .quad 0
ap_gdt_ptr:     .quad 0
ap_idt_ptr:     .quad 0
ap_stack_top:   .quad 0
ap_entry_point: .quad 0
ap_cpu_id:      .long 0
ap_apic_id:     .byte 0
                .byte 0
                .byte 0
                .byte 0

ap_trampoline_end: