initSidebarItems({"fn":[["enable_kernel_page_table","切换到 KERNEL_MEMORY_SET 中的页表。 每个核启动时都需要调用"],["handle_kernel_page_fault","处理来自内核的异常中断 当前还没有需要处理的中断"],["init_kernel_memory_set","初始化 MemorySet，加载所有内存段"],["map_kernel_regions","为 ms 映射内存段的地址。ms 本身一般是用户态的"],["new_memory_set_for_task","创建一个新的用户进程对应的内存映射。它的内核态可以访问内核态的所有映射，但不能修改"]],"struct":[["KERNEL_MEMORY_SET",""],["MemorySet","内存段和相关的页表"]]});