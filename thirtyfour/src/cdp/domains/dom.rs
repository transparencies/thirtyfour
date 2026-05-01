//! `DOM` domain — node tree, querying, identifier mapping.

use serde::{Deserialize, Serialize};

use crate::cdp::Cdp;
use crate::cdp::command::{CdpCommand, Empty};
use crate::cdp::ids::{BackendNodeId, NodeId, RemoteObjectId};
use crate::error::WebDriverResult;

/// `DOM.enable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Enable;
impl CdpCommand for Enable {
    const METHOD: &'static str = "DOM.enable";
    type Returns = Empty;
}

/// `DOM.disable`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Disable;
impl CdpCommand for Disable {
    const METHOD: &'static str = "DOM.disable";
    type Returns = Empty;
}

/// `DOM.getDocument`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocument {
    /// Maximum depth of the tree returned. `-1` for entire subtree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    /// Pierce shadow roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Subset of `DOM.Node` used in responses; full structure has many fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomNode {
    /// Node identifier (short-lived, valid only while DOM is enabled).
    pub node_id: NodeId,
    /// Backend node identifier (stable across reattachment).
    pub backend_node_id: BackendNodeId,
    /// Node type per <https://dom.spec.whatwg.org/#dom-node-nodetype>.
    pub node_type: u32,
    /// Node name (e.g. `"DIV"`).
    pub node_name: String,
    /// Local name.
    pub local_name: String,
    /// Node value.
    #[serde(default)]
    pub node_value: String,
    /// Child node count.
    pub child_node_count: Option<u32>,
    /// Children (when included).
    pub children: Option<Vec<DomNode>>,
    /// Attributes (interleaved name/value pairs).
    pub attributes: Option<Vec<String>>,
    /// Document URL (for the document node). CDP uses SCREAMING `URL`.
    #[serde(rename = "documentURL")]
    pub document_url: Option<String>,
    /// Base URL. CDP uses SCREAMING `URL`.
    #[serde(rename = "baseURL")]
    pub base_url: Option<String>,
}

/// Response for [`GetDocument`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetDocumentResult {
    /// Resulting node.
    pub root: DomNode,
}

impl CdpCommand for GetDocument {
    const METHOD: &'static str = "DOM.getDocument";
    type Returns = GetDocumentResult;
}

/// `DOM.querySelector`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelector {
    /// Node id to start the search at.
    pub node_id: NodeId,
    /// CSS selector.
    pub selector: String,
}

/// Response for [`QuerySelector`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorResult {
    /// Matched node id, or `0` if no node was found.
    pub node_id: NodeId,
}

impl CdpCommand for QuerySelector {
    const METHOD: &'static str = "DOM.querySelector";
    type Returns = QuerySelectorResult;
}

/// `DOM.querySelectorAll`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAll {
    /// Node id to start the search at.
    pub node_id: NodeId,
    /// CSS selector.
    pub selector: String,
}

/// Response for [`QuerySelectorAll`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllResult {
    /// All matched node ids.
    pub node_ids: Vec<NodeId>,
}

impl CdpCommand for QuerySelectorAll {
    const METHOD: &'static str = "DOM.querySelectorAll";
    type Returns = QuerySelectorAllResult;
}

/// `DOM.describeNode` — returns metadata for a node identified by id, backend
/// id, or remote object id.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNode {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JS object handle for the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
    /// Maximum depth of the subtree returned. `-1` for entire subtree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    /// Pierce shadow roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Response for [`DescribeNode`].
#[derive(Debug, Clone, Deserialize)]
pub struct DescribeNodeResult {
    /// Node description.
    pub node: DomNode,
}

impl CdpCommand for DescribeNode {
    const METHOD: &'static str = "DOM.describeNode";
    type Returns = DescribeNodeResult;
}

/// `DOM.requestNode` — requests a `NodeId` for an object reference.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNode {
    /// JS object handle to request a node for.
    pub object_id: RemoteObjectId,
}

/// Response for [`RequestNode`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNodeResult {
    /// Node id of the requested object.
    pub node_id: NodeId,
}

impl CdpCommand for RequestNode {
    const METHOD: &'static str = "DOM.requestNode";
    type Returns = RequestNodeResult;
}

/// `DOM.resolveNode` — get a `RemoteObject` handle for a node.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNode {
    /// Node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// Object group name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Execution context id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<crate::cdp::ids::ExecutionContextId>,
}

/// Response for [`ResolveNode`].
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveNodeResult {
    /// JS object wrapper for the node.
    pub object: super::runtime::RemoteObject,
}

impl CdpCommand for ResolveNode {
    const METHOD: &'static str = "DOM.resolveNode";
    type Returns = ResolveNodeResult;
}

/// `DOM.getBoxModel`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModel {
    /// Node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JS object handle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}

/// Response for [`GetBoxModel`].
#[derive(Debug, Clone, Deserialize)]
pub struct GetBoxModelResult {
    /// Box model description.
    pub model: serde_json::Value,
}

impl CdpCommand for GetBoxModel {
    const METHOD: &'static str = "DOM.getBoxModel";
    type Returns = GetBoxModelResult;
}

/// `DOM.scrollIntoViewIfNeeded`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollIntoViewIfNeeded {
    /// Node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend node id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JS object handle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<RemoteObjectId>,
}
impl CdpCommand for ScrollIntoViewIfNeeded {
    const METHOD: &'static str = "DOM.scrollIntoViewIfNeeded";
    type Returns = Empty;
}

/// Domain facade returned by [`Cdp::dom`].
#[derive(Debug)]
pub struct DomDomain<'a> {
    cdp: &'a Cdp,
}

impl<'a> DomDomain<'a> {
    pub(crate) fn new(cdp: &'a Cdp) -> Self {
        Self {
            cdp,
        }
    }

    /// `DOM.enable`.
    pub async fn enable(&self) -> WebDriverResult<()> {
        self.cdp.send(Enable).await?;
        Ok(())
    }

    /// `DOM.disable`.
    pub async fn disable(&self) -> WebDriverResult<()> {
        self.cdp.send(Disable).await?;
        Ok(())
    }

    /// `DOM.getDocument` (depth `-1`, no pierce).
    pub async fn get_document(&self) -> WebDriverResult<DomNode> {
        let r = self
            .cdp
            .send(GetDocument {
                depth: Some(-1),
                pierce: None,
            })
            .await?;
        Ok(r.root)
    }

    /// `DOM.querySelector`.
    pub async fn query_selector(
        &self,
        node_id: NodeId,
        selector: impl Into<String>,
    ) -> WebDriverResult<NodeId> {
        let r = self
            .cdp
            .send(QuerySelector {
                node_id,
                selector: selector.into(),
            })
            .await?;
        Ok(r.node_id)
    }

    /// `DOM.querySelectorAll`.
    pub async fn query_selector_all(
        &self,
        node_id: NodeId,
        selector: impl Into<String>,
    ) -> WebDriverResult<Vec<NodeId>> {
        let r = self
            .cdp
            .send(QuerySelectorAll {
                node_id,
                selector: selector.into(),
            })
            .await?;
        Ok(r.node_ids)
    }

    /// `DOM.describeNode` for a remote-object id.
    pub async fn describe_node_for_object(
        &self,
        object_id: RemoteObjectId,
    ) -> WebDriverResult<DomNode> {
        let r = self
            .cdp
            .send(DescribeNode {
                object_id: Some(object_id),
                ..Default::default()
            })
            .await?;
        Ok(r.node)
    }

    /// `DOM.requestNode`.
    pub async fn request_node(&self, object_id: RemoteObjectId) -> WebDriverResult<NodeId> {
        let r = self
            .cdp
            .send(RequestNode {
                object_id,
            })
            .await?;
        Ok(r.node_id)
    }
}
