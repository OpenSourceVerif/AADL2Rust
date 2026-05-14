use aadl_intermediate::*;

/// Merge and deduplicate StructDef items in a RustModule
pub fn merge_item_defs(module: RustModule) -> RustModule {
    let mut items = module.items;
    let mut i = 0;

    // println!("Start merging duplicate struct definitions...");

    while i < items.len() {
        let current_name = match &items[i] {
            Item::Struct(s) => Some(s.name.clone()),
            _ => None,
        };

        if let Some(name) = current_name {
            let mut j = i + 1;

            while j < items.len() {
                let is_same_struct = match &items[j] {
                    Item::Struct(s) => s.name == name,
                    _ => false,
                };

                if is_same_struct {
                    // Use split_at_mut to safely obtain two mutable references
                    let (left, right) = items.split_at_mut(j);
                    if let Item::Struct(target) = &mut left[i] {
                        if let Item::Struct(source) = &right[0] {
                            // Print information about the struct to be merged/removed
                            // println!("Merging duplicate struct: {}", name);
                            // println!("Struct definition to be removed at index: {}", j);
                            // println!("Source struct field count: {}", source.fields.len());
                            // Create a full clone of source to avoid borrowing issues
                            let source_clone = source.clone();
                            merge_single_struct(target, &source_clone);
                        }
                    }
                    let removed = items.remove(j);
                    if let Item::Struct(_removed_struct) = removed {
                        // println!("-----------------------------Successfully removed duplicate struct--------------------------: {}", removed_struct.name);
                        // println!("----------------------------------------");
                    }
                    continue;
                }
                j += 1;
            }
        }
        i += 1;
    }

    // println!("Struct merge completed, {} items retained", items.len());

    RustModule {
        name: module.name,
        docs: module.docs,
        items,
        attrs: module.attrs,
        vis: module.vis,
    }
}

/// Merge two StructDef instances with the same name
fn merge_single_struct(target: &mut StructDef, source: &StructDef) {
    // Merge fields (deduplicate by name)
    let _original_field_count = target.fields.len();
    for src_field in source.fields.iter().cloned() {
        if !target.fields.iter().any(|f| f.name == src_field.name) {
            target.fields.push(src_field);
        }
    }
    // println!(
    //     "Merged fields: original {} + added {} = now {}",
    //     original_field_count,
    //     source.fields.len(),
    //     target.fields.len()
    // );

    // Merge generics (deduplicate by name)
    for src_generic in source.generics.iter().cloned() {
        if !target.generics.iter().any(|g| g.name == src_generic.name) {
            target.generics.push(src_generic);
        }
    }

    // Merge derives (simple deduplication)
    for derive in &source.derives {
        if !target.derives.contains(derive) {
            target.derives.push(derive.clone());
        }
    }

    // Merge docs (concatenate directly)
    // target.docs.extend(source.docs.clone());

    // Preserve the more permissive visibility
    if let Visibility::Public = source.vis {
        target.vis = Visibility::Public;
    }
}
