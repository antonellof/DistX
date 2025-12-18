# Adapted from https://docs.streamlit.io/knowledge-base/tutorials/build-conversational-apps#build-a-simple-chatbot-gui-with-streaming
import os
import base64
import gc
import tempfile
import uuid

from dotenv import load_dotenv

import streamlit as st

# Now using OpenAI embeddings - no heavy torch/transformers imports!
from rag_code import EmbedData, DistXVDB, Retriever, RAG, extract_text_from_file
from qdrant_client import QdrantClient

if "id" not in st.session_state:
    st.session_state.id = uuid.uuid4()
    st.session_state.file_cache = {}
    st.session_state.existing_collections = []

session_id = st.session_state.id
collection_name = "chat with docs"
batch_size = 32

load_dotenv()

def get_distx_status():
    """Check DistX connection and get existing collections."""
    try:
        client = QdrantClient(url="http://localhost:6333", prefer_grpc=False, check_compatibility=False, timeout=5)
        collections = client.get_collections()
        print(f"[DEBUG] Found {len(collections.collections)} collections")
        for c in collections.collections:
            print(f"[DEBUG] Collection: {c.name}")
        return {
            "connected": True,
            "collections": [
                {
                    "name": c.name,
                }
                for c in collections.collections
            ]
        }
    except Exception as e:
        print(f"[DEBUG] DistX connection error: {e}")
        return {"connected": False, "error": str(e), "collections": []}

def get_collection_info(collection_name):
    """Get info about a specific collection."""
    try:
        client = QdrantClient(url="http://localhost:6333", prefer_grpc=False, check_compatibility=False, timeout=5)
        info = client.get_collection(collection_name)
        print(f"[DEBUG] Collection '{collection_name}': {info.points_count} points, status={info.status}")
        return {
            "name": collection_name,
            "points_count": info.points_count,
            "status": info.status
        }
    except Exception as e:
        print(f"[DEBUG] Error getting collection info: {e}")
        return None

def get_documents_in_collection(collection_name):
    """Get list of documents in a collection with their chunk counts."""
    try:
        distx_vdb = DistXVDB(collection_name=collection_name, batch_size=32, vector_dim=1536)
        distx_vdb.define_client()
        return distx_vdb.list_documents()
    except Exception as e:
        print(f"[DEBUG] Error listing documents: {e}")
        return []

def delete_document_from_collection(collection_name, document_name):
    """Delete a specific document from a collection."""
    try:
        distx_vdb = DistXVDB(collection_name=collection_name, batch_size=32, vector_dim=1536)
        distx_vdb.define_client()
        return distx_vdb.delete_document(document_name)
    except Exception as e:
        print(f"[DEBUG] Error deleting document: {e}")
        return False

def load_existing_collection(collection_name):
    """Load an existing collection for querying."""
    try:
        distx_vdb = DistXVDB(collection_name=collection_name, batch_size=batch_size, vector_dim=1536)
        distx_vdb.define_client()
        
        # Create embeddata for query embedding
        embeddata = EmbedData(batch_size=batch_size)
        
        retriever = Retriever(vector_db=distx_vdb, embeddata=embeddata)
        llm_name = os.getenv("LLM_MODEL", "gpt-5-mini")
        
        query_engine = RAG(retriever=retriever, llm_name=llm_name)
        return query_engine
    except Exception as e:
        st.error(f"Error loading collection: {e}")
        return None

def reset_chat():
    st.session_state.messages = []
    st.session_state.context = None
    gc.collect()


def display_pdf(file):
    """Display PDF preview in sidebar."""
    st.markdown("### PDF Preview")
    base64_pdf = base64.b64encode(file.read()).decode("utf-8")
    pdf_display = f"""<iframe src="data:application/pdf;base64,{base64_pdf}" width="400" height="100%" type="application/pdf"
                        style="height:100vh; width:100%"
                    >
                    </iframe>"""
    st.markdown(pdf_display, unsafe_allow_html=True)


with st.sidebar:
    # Check DistX status and show existing collections
    distx_status = get_distx_status()
    
    if distx_status["connected"]:
        col1, col2 = st.columns([3, 1])
        with col1:
            st.success("🟢 DistX Connected")
        with col2:
            if st.button("🔄", help="Refresh"):
                st.rerun()
        
        if distx_status["collections"]:
            st.subheader("📚 Existing Collections")
            
            for coll in distx_status["collections"]:
                coll_info = get_collection_info(coll["name"])
                if coll_info:
                    with st.expander(f"📁 {coll['name']} ({coll_info['points_count']} chunks)", expanded=True):
                        # Show documents in this collection
                        documents = get_documents_in_collection(coll["name"])
                        
                        if documents:
                            st.caption(f"📄 {len(documents)} document(s):")
                            for doc in documents:
                                doc_col1, doc_col2 = st.columns([4, 1])
                                with doc_col1:
                                    st.text(f"• {doc['name']} ({doc['chunks']} chunks)")
                                with doc_col2:
                                    if st.button("🗑️", key=f"del_doc_{coll['name']}_{doc['name']}", help=f"Delete {doc['name']}"):
                                        if delete_document_from_collection(coll['name'], doc['name']):
                                            st.success(f"Deleted {doc['name']}")
                                            st.rerun()
                                        else:
                                            st.error("Delete failed")
                        else:
                            st.caption("No documents tracked (legacy data)")
                        
                        st.divider()
                        
                        col1, col2 = st.columns(2)
                        with col1:
                            if st.button("✅ Use", key=f"use_{coll['name']}"):
                                query_engine = load_existing_collection(coll['name'])
                                if query_engine:
                                    st.session_state.file_cache[f"existing-{coll['name']}"] = query_engine
                                    st.success(f"Loaded!")
                                    st.rerun()
                        
                        with col2:
                            if st.button("🗑️ Delete All", key=f"del_{coll['name']}"):
                                try:
                                    client = QdrantClient(url="http://localhost:6333", prefer_grpc=False, check_compatibility=False)
                                    client.delete_collection(coll['name'])
                                    cache_key = f"existing-{coll['name']}"
                                    if cache_key in st.session_state.file_cache:
                                        del st.session_state.file_cache[cache_key]
                                    st.success("Deleted!")
                                    st.rerun()
                                except Exception as e:
                                    st.error(f"Error: {e}")
        else:
            st.info("No collections yet. Upload documents to get started!")
    else:
        st.error("🔴 DistX Not Connected")
        st.caption(f"Error: {distx_status.get('error', 'Unknown')}")
        st.caption("Make sure DistX is running on port 6333")
    
    st.divider()
    
    st.header("📄 Add new documents")
    st.caption("Supports: PDF, Word (.docx), Text (.txt)")
    
    # Accept multiple files of different types
    uploaded_files = st.file_uploader(
        "Choose files", 
        type=["pdf", "docx", "txt"],
        accept_multiple_files=True
    )

    if uploaded_files:
        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                all_texts = []
                file_names = []
                
                for uploaded_file in uploaded_files:
                    file_path = os.path.join(temp_dir, uploaded_file.name)
                    
                    with open(file_path, "wb") as f:
                        f.write(uploaded_file.getvalue())
                    
                    file_names.append(uploaded_file.name)
                
                # Create a combined key for all files
                file_key = f"{session_id}-{'-'.join(sorted(file_names))}"

                if file_key not in st.session_state.get('file_cache', {}):
                    
                    # Set up vector database (DistX) first
                    distx_vdb = DistXVDB(collection_name=collection_name,
                                         batch_size=batch_size,
                                         vector_dim=1536)
                    distx_vdb.define_client()
                    distx_vdb.create_collection()
                    
                    # Process each file separately to track document names
                    for uploaded_file in uploaded_files:
                        file_path = os.path.join(temp_dir, uploaded_file.name)
                        
                        try:
                            with st.spinner(f"Processing {uploaded_file.name}..."):
                                # Extract text
                                text = extract_text_from_file(file_path)
                                if not text or len(text.strip()) < 10:
                                    st.warning(f"⚠ {uploaded_file.name}: No text extracted")
                                    continue
                                
                                st.write(f"✓ Extracted {len(text):,} chars from {uploaded_file.name}")
                                
                                # Embed this document
                                embeddata = EmbedData(batch_size=batch_size)
                                embeddata.embed([text])
                                
                                # Ingest with document name
                                distx_vdb.ingest_data(embeddata=embeddata, document_name=uploaded_file.name)
                                
                                st.write(f"✓ Indexed {uploaded_file.name}")
                                
                        except Exception as e:
                            st.error(f"✗ {uploaded_file.name}: {e}")

                    # Create embeddata for querying (reuse last one or create new)
                    embeddata = EmbedData(batch_size=batch_size)
                    
                    # set up retriever and RAG
                    with st.spinner("Setting up RAG system..."):
                        retriever = Retriever(vector_db=distx_vdb, embeddata=embeddata)
                        llm_name = os.getenv("LLM_MODEL", "gpt-5-mini")
                        
                        query_engine = RAG(
                            retriever=retriever, 
                            llm_name=llm_name
                        )

                    st.session_state.file_cache[file_key] = query_engine
                    st.success("✅ Ready to Chat!")

                else:
                    st.success("✅ Documents already indexed. Ready to Chat!")

                # Show PDF preview for PDF files
                for uploaded_file in uploaded_files:
                    if uploaded_file.name.lower().endswith('.pdf'):
                        with st.expander(f"Preview: {uploaded_file.name}"):
                            uploaded_file.seek(0)  # Reset file pointer
                            display_pdf(uploaded_file)
                            
        except Exception as e:
            st.error(f"An error occurred: {e}")
            import traceback
            st.code(traceback.format_exc())
            st.stop()     

col1, col2 = st.columns([6, 1])

with col1:
    llm_model = os.getenv("LLM_MODEL", "gpt-5-mini")
    st.header(f"RAG Stack with DistX and OpenAI ({llm_model})")

with col2:
    st.button("Clear ↺", on_click=reset_chat)

# Initialize chat history
if "messages" not in st.session_state:
    reset_chat()

# Show status if no document loaded
if not st.session_state.get('file_cache'):
    distx_status = get_distx_status()
    if distx_status["connected"] and distx_status["collections"]:
        st.info("👆 Select an existing collection or upload new documents in the sidebar to get started!")
    else:
        st.info("👆 Upload documents (PDF, Word, or text files) in the sidebar to get started!")

# Display chat messages from history on app rerun
for message in st.session_state.messages:
    with st.chat_message(message["role"]):
        st.markdown(message["content"])


# Accept user input
if prompt := st.chat_input("What's up?"):
    # Check if a document has been uploaded and indexed
    if "file_cache" not in st.session_state or not st.session_state.file_cache:
        st.warning("Please upload documents first to start chatting!")
        st.stop()
    
    # Get the query engine from cache (should exist if file was uploaded)
    file_keys = list(st.session_state.file_cache.keys())
    if not file_keys:
        st.warning("Please upload documents first!")
        st.stop()
    
    query_engine = st.session_state.file_cache[file_keys[0]]
    
    # Add user message to chat history
    st.session_state.messages.append({"role": "user", "content": prompt})
    # Display user message in chat message container
    with st.chat_message("user"):
        st.markdown(prompt)

    # Display assistant response in chat message container
    with st.chat_message("assistant"):
        message_placeholder = st.empty()
        full_response = ""
        
        try:
            # Stream response from LLM
            streaming_response = query_engine.query(prompt)
            
            for chunk in streaming_response:
                try:
                    # Handle llama-index OpenAI streaming format
                    # stream_complete returns chunks with 'delta' attribute containing text
                    if hasattr(chunk, 'delta'):
                        new_text = chunk.delta or ""
                    elif hasattr(chunk, 'raw'):
                        # Fallback: extract from raw OpenAI format
                        try:
                            new_text = chunk.raw.get("choices", [{}])[0].get("delta", {}).get("content", "")
                        except (KeyError, IndexError, AttributeError):
                            new_text = ""
                    elif hasattr(chunk, 'text'):
                        new_text = chunk.text or ""
                    else:
                        # Last resort: convert to string
                        new_text = str(chunk) if chunk else ""
                    
                    if new_text:
                        full_response += new_text
                        message_placeholder.markdown(full_response + "▌")
                except Exception as e:
                    # Log error for debugging but continue
                    st.warning(f"Streaming chunk error: {e}")
                    pass

            message_placeholder.markdown(full_response)
        except Exception as e:
            st.error(f"Error generating response: {e}")
            import traceback
            st.code(traceback.format_exc())

    # Add assistant response to chat history
    st.session_state.messages.append({"role": "assistant", "content": full_response})