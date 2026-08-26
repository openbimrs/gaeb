use roxmltree::{Document, Node, NodeId};

pub(super) struct Tree<'a> {
    pub(super) document: Document<'a>,
    namespace: String,
}

impl<'a> Tree<'a> {
    pub(super) fn parse(bytes: &'a [u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| error.to_string())?
            .strip_prefix('\u{feff}')
            .unwrap_or_else(|| std::str::from_utf8(bytes).expect("UTF-8 was checked"));
        let document = Document::parse(text).map_err(|error| error.to_string())?;
        let namespace = document
            .root_element()
            .tag_name()
            .namespace()
            .unwrap_or_default()
            .to_owned();
        Ok(Self {
            document,
            namespace,
        })
    }

    pub(super) fn root(&self) -> NodeId {
        self.document.root_element().id()
    }

    pub(super) fn all(&self, local_name: &str) -> Vec<NodeId> {
        self.document
            .descendants()
            .filter(|node| self.is(node.id(), local_name))
            .map(|node| node.id())
            .collect()
    }

    pub(super) fn all_named(&self, local_name: &str) -> Vec<NodeId> {
        self.all(local_name)
    }

    pub(super) fn is(&self, node: NodeId, local_name: &str) -> bool {
        self.element(node).is_some_and(|element| {
            element.tag_name().name() == local_name
                && element.tag_name().namespace().unwrap_or_default() == self.namespace
        })
    }

    pub(super) fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.node(node)?.parent().map(|parent| parent.id())
    }

    pub(super) fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.node(node)
            .map(|node| node.children().map(|child| child.id()).collect())
            .unwrap_or_default()
    }

    pub(super) fn child(&self, node: NodeId, local_name: &str) -> Option<NodeId> {
        self.node(node)?
            .children()
            .find(|child| self.is(child.id(), local_name))
            .map(|child| child.id())
    }

    pub(super) fn first_child(&self, node: NodeId, local_name: &str) -> Option<NodeId> {
        self.child(node, local_name)
    }

    pub(super) fn element(&self, node: NodeId) -> Option<Node<'_, 'a>> {
        self.node(node).filter(|node| node.is_element())
    }

    pub(super) fn text(&self, node: NodeId) -> String {
        self.node(node)
            .into_iter()
            .flat_map(|node| node.descendants())
            .filter(|node| node.is_text())
            .filter_map(|node| node.text())
            .collect::<String>()
            .trim()
            .to_owned()
    }

    pub(super) fn child_text(&self, node: NodeId, local_name: &str) -> Option<String> {
        self.child(node, local_name).map(|child| self.text(child))
    }

    pub(super) fn attribute(&self, node: NodeId, local_name: &str) -> Option<&str> {
        self.element(node)?.attribute(local_name)
    }

    pub(super) fn attribute_signature(&self, node: NodeId) -> String {
        let mut attributes: Vec<_> = self
            .element(node)
            .into_iter()
            .flat_map(|element| element.attributes())
            .map(|attribute| {
                (
                    attribute.namespace().unwrap_or_default().to_owned(),
                    attribute.name().to_owned(),
                    attribute.value().to_owned(),
                )
            })
            .collect();
        attributes.sort_unstable();
        attributes
            .into_iter()
            .map(|(namespace, name, value)| format!("{namespace}\u{1f}{name}\u{1f}{value}"))
            .collect::<Vec<_>>()
            .join("\u{1e}")
    }

    pub(super) fn location(&self, node: NodeId) -> String {
        let mut parts = Vec::new();
        let mut current = self.node(node);
        while let Some(element) = current.filter(|node| node.is_element()) {
            let mut part = element.tag_name().name().to_owned();
            if let Some(value) = element.attribute("ID") {
                part.push_str("[@ID='");
                part.push_str(value);
                part.push_str("']");
            }
            parts.push(part);
            current = element.parent();
        }
        parts.reverse();
        format!("/{}", parts.join("/"))
    }

    fn node(&self, node: NodeId) -> Option<Node<'_, 'a>> {
        self.document.get_node(node)
    }
}
