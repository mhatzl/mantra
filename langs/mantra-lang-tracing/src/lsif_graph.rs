use std::collections::HashMap;

use lsp_types::lsif;

// struct ItemEdgeIn {
//     doc_id: lsif::Id,
//     /// ReferenceResult-ID
//     out_id: lsif::Id,
// }

struct ItemEdgeOut {
    doc_id: lsif::Id,
    range_ids: Vec<lsif::Id>,
}

#[derive(Debug, Hash, PartialEq, PartialOrd, Eq, Clone)]
pub struct Item {
    name: String,
    filepath: String,
    start_line: u32,
    end_line: u32,
}

#[derive(Debug, Hash, PartialEq, PartialOrd, Eq, Clone)]
pub struct FileLocation {
    filepath: String,
    start_line: u32,
    end_line: u32,
}

struct ItemRefResult {
    range_ids: Vec<lsif::Id>,
    ref_result_id: lsif::Id,
}

pub struct LsifGraph {
    project_root: Option<String>,
    elements: HashMap<lsif::Id, lsif::Element>,
    documents: HashMap<String, lsif::Id>,
    idents: HashMap<String, lsif::Id>,
    // /// Range-ID points to Doc-ID
    // contains_in: HashMap<lsif::Id, lsif::Id>,
    // /// Doc-ID points to Range-IDs
    // contains_out: HashMap<lsif::Id, Vec<lsif::Id>>,
    // /// ResultSet-ID points to Range-ID
    // result_set_ranges: HashMap<lsif::Id, Vec<lsif::Id>>,
    /// Range-ID points to ResultSet-ID
    range_to_result_set: HashMap<lsif::Id, lsif::Id>,
    /// MonikerVertex-ID points to ResultSet-ID
    moniker_in: HashMap<lsif::Id, lsif::Id>,
    /// ResultSet-ID points to MonikerVertex-ID
    moniker_out: HashMap<lsif::Id, lsif::Id>,
    /// ReferenceResult-ID points to ResultSet-ID
    reference_in: HashMap<lsif::Id, lsif::Id>,
    /// ResultSet-ID points to ReferenceResult-ID
    reference_out: HashMap<lsif::Id, lsif::Id>,
    // /// Referenced Range-ID points to Doc-ID and ReferenceResult-ID
    // item_reference_in: HashMap<lsif::Id, ItemEdgeIn>,
    /// ReferenceResult-ID points to Doc-IDs with referenced Range-IDs
    item_reference_out: HashMap<lsif::Id, Vec<ItemEdgeOut>>,
    // /// Definition Range-ID points to Doc-ID and ReferenceResult-ID
    // item_definition_in: HashMap<lsif::Id, ItemEdgeIn>,
    /// ReferenceResult-ID points to Doc-ID and definition Range-IDs
    ///
    /// **Note:** This assumes that at most one document contains the definition of an item.
    item_definition_out: HashMap<lsif::Id, ItemEdgeOut>,
    /// Doc-ID points to Range-IDs and ReferenceResult-ID of items defined in the doc
    doc_def_items: HashMap<lsif::Id, Vec<ItemRefResult>>,
}

impl LsifGraph {
    pub fn create(lsif_content: &str) -> Result<Self, serde_json::Error> {
        let nr_elems = lsif_content.lines().count();

        let mut elements = HashMap::with_capacity(nr_elems);
        let mut documents = HashMap::new();
        let mut idents = HashMap::new();
        // let mut contains_in = HashMap::new();
        // let mut contains_out = HashMap::new();
        let mut result_set_ranges = HashMap::new();
        let mut range_to_result_set = HashMap::new();
        let mut moniker_in = HashMap::new();
        let mut moniker_out = HashMap::new();
        let mut reference_in = HashMap::new();
        let mut reference_out = HashMap::new();
        // let mut item_reference_in = HashMap::new();
        let mut item_reference_out = HashMap::new();
        // let mut item_definition_in = HashMap::new();
        let mut item_definition_out = HashMap::new();
        let mut doc_def_items = HashMap::new();

        let mut project_root = None;

        for line in lsif_content.lines() {
            let entry = serde_json::from_str::<lsif::Entry>(line)?;

            if let lsif::Element::Vertex(lsif::Vertex::Document(doc)) = &entry.data {
                documents.insert(doc.uri.as_str().to_string(), entry.id.clone());
            } else if let lsif::Element::Vertex(lsif::Vertex::Moniker(moniker)) = &entry.data {
                idents.insert(moniker.identifier.clone(), entry.id.clone());
            }
            // else if let lsif::Element::Edge(lsif::Edge::Contains(edge)) = &entry.data {
            //     for in_v in &edge.in_vs {
            //         contains_in.insert(in_v.clone(), edge.out_v.clone());
            //     }

            //     contains_out.insert(edge.out_v.clone(), edge.in_vs.clone());
            // }
            else if let lsif::Element::Edge(lsif::Edge::Next(next)) = &entry.data {
                result_set_ranges
                    .entry(next.in_v.clone())
                    .and_modify(|range_ids: &mut Vec<lsp_types::NumberOrString>| {
                        range_ids.push(next.out_v.clone())
                    })
                    .or_insert(vec![next.out_v.clone()]);
                range_to_result_set.insert(next.out_v.clone(), next.in_v.clone());
            } else if let lsif::Element::Edge(lsif::Edge::Moniker(moniker)) = &entry.data {
                moniker_in.insert(moniker.in_v.clone(), moniker.out_v.clone());
                moniker_out.insert(moniker.out_v.clone(), moniker.in_v.clone());
            } else if let lsif::Element::Edge(lsif::Edge::References(ref_edge)) = &entry.data {
                reference_in.insert(ref_edge.in_v.clone(), ref_edge.out_v.clone());
                reference_out.insert(ref_edge.out_v.clone(), ref_edge.in_v.clone());
            } else if let lsif::Element::Edge(lsif::Edge::Item(item_edge)) = &entry.data {
                if item_edge.property == Some(lsif::ItemKind::References) {
                    // for in_id in &item_edge.edge_data.in_vs {
                    //     item_reference_in.insert(
                    //         in_id.clone(),
                    //         ItemEdgeIn {
                    //             doc_id: item_edge.document.clone(),
                    //             out_id: item_edge.edge_data.out_v.clone(),
                    //         },
                    //     );
                    // }

                    item_reference_out
                        .entry(item_edge.edge_data.out_v.clone())
                        .and_modify(|refs: &mut Vec<ItemEdgeOut>| {
                            refs.push(ItemEdgeOut {
                                doc_id: item_edge.document.clone(),
                                range_ids: item_edge.edge_data.in_vs.clone(),
                            })
                        })
                        .or_insert(vec![ItemEdgeOut {
                            doc_id: item_edge.document.clone(),
                            range_ids: item_edge.edge_data.in_vs.clone(),
                        }]);
                } else if item_edge.property == Some(lsif::ItemKind::Definitions) {
                    // for in_id in &item_edge.edge_data.in_vs {
                    //     item_definition_in.insert(
                    //         in_id.clone(),
                    //         ItemEdgeIn {
                    //             doc_id: item_edge.document.clone(),
                    //             out_id: item_edge.edge_data.out_v.clone(),
                    //         },
                    //     );
                    // }

                    item_definition_out.insert(
                        item_edge.edge_data.out_v.clone(),
                        ItemEdgeOut {
                            doc_id: item_edge.document.clone(),
                            range_ids: item_edge.edge_data.in_vs.clone(),
                        },
                    );

                    doc_def_items
                        .entry(item_edge.document.clone())
                        .and_modify(|defs: &mut Vec<ItemRefResult>| {
                            defs.push(ItemRefResult {
                                range_ids: item_edge.edge_data.in_vs.clone(),
                                ref_result_id: item_edge.edge_data.out_v.clone(),
                            })
                        })
                        .or_insert(vec![ItemRefResult {
                            range_ids: item_edge.edge_data.in_vs.clone(),
                            ref_result_id: item_edge.edge_data.out_v.clone(),
                        }]);
                }
            } else if let lsif::Element::Vertex(lsif::Vertex::MetaData(meta)) = &entry.data {
                project_root = if meta.project_root.as_str().ends_with('/') {
                    Some(meta.project_root.as_str().to_string())
                } else {
                    Some(format!("{}/", meta.project_root.as_str()))
                };
            }

            elements.insert(entry.id.clone(), entry.data);
        }

        Ok(Self {
            project_root,
            elements,
            documents,
            idents,
            // contains_in,
            // contains_out,
            // result_set_ranges,
            range_to_result_set,
            moniker_in,
            moniker_out,
            reference_in,
            reference_out,
            // item_reference_in,
            item_reference_out,
            // item_definition_in,
            item_definition_out,
            doc_def_items,
        })
    }

    pub fn ident_references(&self, identifier: &str) -> Vec<(String, u32)> {
        let mut ref_locations = Vec::new();

        let Some(moniker_id) = self.idents.get(identifier) else {
            return ref_locations;
        };
        let Some(result_set_id) = self.moniker_in.get(moniker_id) else {
            return ref_locations;
        };
        let Some(ref_result_id) = self.reference_out.get(result_set_id) else {
            return ref_locations;
        };
        let Some(item_refs) = self.item_reference_out.get(ref_result_id) else {
            return ref_locations;
        };

        for item_ref in item_refs {
            let doc_element = self.lookup_element(&item_ref.doc_id);
            let filepath = match doc_element {
                lsif::Element::Vertex(lsif::Vertex::Document(doc)) => {
                    let path = doc.uri.as_str().to_string();

                    if let Some(root) = &self.project_root {
                        path.strip_prefix(root).unwrap_or(&path).to_string()
                    } else {
                        path.to_string()
                    }
                }
                _ => unreachable!("Entry is a document."),
            };

            for range_id in &item_ref.range_ids {
                let range_element = self.lookup_element(range_id);
                let (start_line, _) = match range_element {
                    lsif::Element::Vertex(lsif::Vertex::Range { range, tag: _ }) => {
                        (range.start.line, range.end.line)
                    }
                    _ => unreachable!("Entry is a range"),
                };

                ref_locations.push((filepath.clone(), start_line));
            }
        }

        ref_locations
    }

    pub fn contained_items(&self, doc: &str) -> Vec<Item> {
        let abs_doc = self.abs_path(doc);

        let mut items = Vec::new();

        if let Some(doc_id) = self.documents.get(&abs_doc) {
            if let Some(item_refs) = self.doc_def_items.get(doc_id) {
                for item_ref in item_refs {
                    let range_element = self.lookup_element(
                        item_ref
                            .range_ids
                            .first()
                            .expect("At least one item definition must be available."),
                    );
                    let (start_line, end_line) = match &range_element {
                        lsif::Element::Vertex(lsif::Vertex::Range { range, tag: _ }) => {
                            (range.start.line, range.end.line)
                        }
                        _ => unreachable!("Entry is a range"),
                    };

                    if let Some(result_set_id) = self.reference_in.get(&item_ref.ref_result_id) {
                        if let Some(moniker_id) = self.moniker_out.get(result_set_id) {
                            let moniker_element = self.lookup_element(moniker_id);

                            let name = match &moniker_element {
                                lsif::Element::Vertex(lsif::Vertex::Moniker(moniker)) => {
                                    moniker.identifier.clone()
                                }
                                _ => unreachable!("Entry is a moniker"),
                            };

                            items.push(Item {
                                name,
                                filepath: doc.to_string(),
                                start_line,
                                end_line,
                            });
                        }
                    }
                }
            }
        }

        items
    }

    pub fn contains_doc(&self, doc: &str) -> bool {
        let abs_doc = self.abs_path(doc);

        self.documents.contains_key(&abs_doc)
    }

    pub fn get_identifier(&self, doc: &str, line: u32) -> Option<String> {
        let abs_doc = self.abs_path(doc);

        let doc_id = self.documents.get(&abs_doc)?;
        let range_ids: Vec<&lsif::Id> = self
            .doc_def_items
            .get(doc_id)?
            .iter()
            .flat_map(|item| &item.range_ids)
            .collect();

        for range_id in range_ids {
            let range_element = self.lookup_element(range_id);
            let range_start_line = match &range_element {
                lsif::Element::Vertex(lsif::Vertex::Range { range, tag: _ }) => range.start.line,
                _ => unreachable!("Entry is a range"),
            };

            if range_start_line == line {
                if let Some(result_set_id) = self.range_to_result_set.get(range_id) {
                    if let Some(moniker_id) = self.moniker_out.get(result_set_id) {
                        let moniker_element = self.lookup_element(moniker_id);

                        let name = match moniker_element {
                            lsif::Element::Vertex(lsif::Vertex::Moniker(moniker)) => {
                                moniker.identifier.clone()
                            }
                            _ => unreachable!("Entry is a moniker"),
                        };

                        return Some(name);
                    }
                }
            }
        }

        None
    }

    pub fn get_ident_location(&self, identifier: &str) -> Option<FileLocation> {
        let moniker_id = self.idents.get(identifier)?;
        let result_set_id = self.moniker_in.get(moniker_id)?;
        let ref_result_id = self.reference_out.get(result_set_id)?;
        let item_definitions = self.item_definition_out.get(ref_result_id)?;

        // Note: most languages only have one item definition, so taking the first should be fine
        let first_def_range_id = item_definitions.range_ids.first()?;

        let range_element = self.lookup_element(first_def_range_id);
        let (range, _tag) = match range_element {
            lsif::Element::Vertex(lsif::Vertex::Range { range, tag }) => (range, tag),
            _ => unreachable!("Entry must be a range."),
        };

        let filepath = match self.lookup_element(&item_definitions.doc_id) {
            lsif::Element::Vertex(lsif::Vertex::Document(doc)) => {
                let path = doc.uri.as_str();

                if let Some(root) = &self.project_root {
                    path.strip_prefix(root)?.to_string()
                } else {
                    path.to_string()
                }
            }
            _ => unreachable!("Contains 'out' points to a document entry."),
        };

        Some(FileLocation {
            filepath,
            start_line: range.start.line,
            end_line: range.end.line,
        })
    }

    fn lookup_element(&self, id: &lsif::Id) -> &lsif::Element {
        self.elements
            .get(id)
            .expect("Lookup for entry must succeed.")
    }

    fn abs_path(&self, path: &str) -> String {
        if let Some(root) = &self.project_root {
            if path.starts_with(root) {
                path.to_string()
            } else if path.starts_with('/') && root.ends_with('/') {
                let stripped_root = root.strip_suffix('/').expect("Root path ends with '/'.");
                format!("{stripped_root}{path}")
            } else if path.starts_with('/') || root.ends_with('/') {
                format!("{root}{path}")
            } else {
                format!("{root}/{path}")
            }
        } else {
            path.to_string()
        }
    }
}

impl std::fmt::Display for LsifGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for doc in self.documents.keys() {
            writeln!(f, "Doc: {doc}")?;
            for item in self.contained_items(doc) {
                writeln!(
                    f,
                    "- {} (line-range: {}-{})",
                    item.name, item.start_line, item.end_line
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FileLocation;
    use super::LsifGraph;

    #[test]
    fn list_items() {
        let lsif = include_str!("lsif_sample.json");
        let graph = LsifGraph::create(lsif).unwrap();

        println!("{graph}");
    }

    #[test]
    fn list_item_references() {
        let lsif = include_str!("lsif_sample.json");
        let graph = LsifGraph::create(lsif).unwrap();
        let inner_references = graph.ident_references("lsif_test::inner");

        for (path, line) in inner_references {
            println!("{path}:{line}");
        }
    }

    #[test]
    fn resolve_ident() {
        let lsif = include_str!("lsif_sample.json");
        let graph = LsifGraph::create(lsif).unwrap();
        let ident = graph.get_identifier("src/main.rs", 8).unwrap();

        assert_eq!(
            &ident, "lsif_test::foo",
            "Identifier not correctly resolved."
        );
    }

    #[test]
    fn resolve_location() {
        let lsif = include_str!("lsif_sample.json");
        let graph = LsifGraph::create(lsif).unwrap();
        let location = graph.get_ident_location("lsif_test::foo").unwrap();

        assert_eq!(
            location,
            FileLocation {
                filepath: "src/main.rs".to_string(),
                start_line: 8,
                end_line: 8
            },
            "Identifier location not correctly resolved."
        );
    }
}
