use std::io;

use crate::fastfield::MultiValuedFastFieldWriter;
use crate::indexer::doc_id_mapping::DocIdMapping;
use crate::postings::postings_writer::SpecializedPostingsWriter;
use crate::postings::recorder::{BufferLender, NothingRecorder, Recorder};
use crate::postings::stacker::Addr;
use crate::postings::{
    FieldSerializer, IndexingContext, IndexingPosition, PostingsWriter, UnorderedTermId,
};
use crate::schema::term::as_json_path_type_value_bytes;
use crate::schema::Type;
use crate::tokenizer::TokenStream;
use crate::{DocId, Term};

use super::stacker::TermHashMap;

#[derive(Default)]
pub(crate) struct JsonPostingsWriter<Rec: Recorder> {
    str_posting_writer: SpecializedPostingsWriter<Rec>,
    non_str_posting_writer: SpecializedPostingsWriter<NothingRecorder>,
}

impl<Rec: Recorder> From<JsonPostingsWriter<Rec>> for Box<dyn PostingsWriter> {
    fn from(json_postings_writer: JsonPostingsWriter<Rec>) -> Box<dyn PostingsWriter> {
        Box::new(json_postings_writer)
    }
}

impl<Rec: Recorder> PostingsWriter for JsonPostingsWriter<Rec> {
    fn mem_usage(&self) -> usize {
        self.str_posting_writer.mem_usage() + self.non_str_posting_writer.mem_usage()
    }

    fn term_map(&self) -> &TermHashMap {
        self.str_posting_writer.term_map()
    }

    fn subscribe(
        &mut self,
        doc: crate::DocId,
        pos: u32,
        term: &crate::Term,
        ctx: &mut IndexingContext,
    ) -> UnorderedTermId {
        self.non_str_posting_writer.subscribe(doc, pos, term, ctx)
    }

    fn index_text(
        &mut self,
        doc_id: DocId,
        token_stream: &mut dyn TokenStream,
        term_buffer: &mut Term,
        ctx: &mut IndexingContext,
        indexing_position: &mut IndexingPosition,
        _fast_field_writer: Option<&mut MultiValuedFastFieldWriter>,
    ) {
        self.str_posting_writer.index_text(
            doc_id,
            token_stream,
            term_buffer,
            ctx,
            indexing_position,
            None,
        );
    }

    /// The actual serialization format is handled by the `PostingsSerializer`.
    fn serialize(
        &self,
        term_addrs: &[(Term<&[u8]>, Addr, UnorderedTermId)],
        doc_id_map: Option<&DocIdMapping>,
        ctx: &IndexingContext,
        serializer: &mut FieldSerializer,
    ) -> io::Result<()> {
        let mut buffer_lender = BufferLender::default();
        for (term, addr, _) in term_addrs {
            // TODO optimization opportunity here.
            if let Some((_, typ, _)) = as_json_path_type_value_bytes(term.value_bytes()) {
                if typ == Type::Str {
                    SpecializedPostingsWriter::<Rec>::serialize_one_term(
                        term,
                        *addr,
                        doc_id_map,
                        &mut buffer_lender,
                        ctx,
                        &self.str_posting_writer.term_map,
                        serializer,
                    )?;
                } else {
                    SpecializedPostingsWriter::<NothingRecorder>::serialize_one_term(
                        term,
                        *addr,
                        doc_id_map,
                        &mut buffer_lender,
                        ctx,
                        &self.str_posting_writer.term_map,
                        serializer,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn total_num_tokens(&self) -> u64 {
        self.str_posting_writer.total_num_tokens() + self.non_str_posting_writer.total_num_tokens()
    }
}
