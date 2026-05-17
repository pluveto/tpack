---
v: 3
title: "TPACK: A Self-Describing Typed Binary Serialization Format"
abbrev: TPACK
docname: draft-zhang-tpack-format-00
category: exp
submissiontype: independent
ipr: trust200902
stand_alone: true
date: 2026-05-16
pi:
  toc: true
  sortrefs: false
  symrefs: true
author:
  -
    ins: Z. Zhang
    name: Zijing Zhang
    org: Independent
    email: zijing.zhang@ry.rs
normative:
  RFC2119:
    override: true
    title: Key words for use in RFCs to Indicate Requirement Levels
    author:
      ins: S. Bradner
      name: Scott Bradner
    date: 1997-03
    target: https://www.rfc-editor.org/info/rfc2119
    seriesinfo:
      BCP: 14
      RFC: 2119
      DOI: 10.17487/RFC2119
  RFC3629:
    override: true
    title: UTF-8, a transformation format of ISO 10646
    author:
      ins: F. Yergeau
      name: Francois Yergeau
    date: 2003-11
    target: https://www.rfc-editor.org/info/rfc3629
    seriesinfo:
      STD: 63
      RFC: 3629
      DOI: 10.17487/RFC3629
  RFC8174:
    override: true
    title: Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words
    author:
      ins: B. Leiba
      name: Barry Leiba
    date: 2017-05
    target: https://www.rfc-editor.org/info/rfc8174
    seriesinfo:
      BCP: 14
      RFC: 8174
      DOI: 10.17487/RFC8174
informative:
  RFC8610:
    title: "Concise Data Definition Language (CDDL): A Notational Convention to Express Concise Binary Object Representation (CBOR) and JSON Data Structures"
    author:
      -
        ins: C. Bormann
        name: Carsten Bormann
      -
        ins: P. Hoffman
        name: Paul Hoffman
    date: 2019-06
    target: https://www.rfc-editor.org/info/rfc8610
    seriesinfo:
      RFC: 8610
      DOI: 10.17487/RFC8610
  RFC8949:
    title: "Concise Binary Object Representation (CBOR)"
    author:
      -
        ins: C. Bormann
        name: Carsten Bormann
      -
        ins: P. Hoffman
        name: Paul Hoffman
    date: 2020-12
    target: https://www.rfc-editor.org/info/rfc8949
    seriesinfo:
      STD: 94
      RFC: 8949
      DOI: 10.17487/RFC8949
  AvroSpec:
    title: Apache Avro Specification
    author:
      name: Apache Avro Project
    target: https://avro.apache.org/docs/1.12.0/specification/
  ArrowFormat:
    title: Apache Arrow Columnar Format and IPC Specification
    author:
      name: Apache Arrow Project
    target: https://arrow.apache.org/docs/format/Columnar.html

--- abstract

   TPACK (Typed Pack) is a strictly typed binary serialization format.
   A TPACK core message embeds the complete schema required to decode
   and validate its data.  TPACK also defines an envelope form that can
   refer to an externally established schema by an opaque SchemaId for
   cached-schema transports.  The format is designed for stateless
   interchange, single-pass parsing, immediate validation, compact
   binary representation, arbitrary-precision decimals, rich temporal
   values, and nested data structures.

--- middle

# Introduction

   Existing binary serialization formats typically optimize for one of
   three properties: compactness, self-description, or strict typing.
   Formats such as MessagePack and CBOR are compact and self-describing
   at the value level, but they do not carry a complete schema capable
   of enforcing structural constraints during parsing.  Formats such as
   Protocol Buffers, FlatBuffers, and Avro provide stronger typing, but
   generally depend on schemas distributed out of band, generated code,
   or registries.

   TPACK defines a different point in the design space.  A TPACK core
   message is a typed value preceded by the complete binary schema
   needed to decode that value.  The receiver of such a message does
   not need an external schema, a table definition, a namespace, or any
   application-specific registry to determine the shape and primitive
   types of the payload.

   For high-frequency streams and other environments where both
   endpoints already share schema state, TPACK also defines a cached
   schema envelope profile.  That profile uses an opaque SchemaId to
   identify a schema established by an enclosing application, stream,
   cache, or registry.  TPACK does not define how SchemaId values are
   generated, negotiated, authenticated, or registered.

   This document specifies TPACK version 1.  TPACK version 1 deliberately
   does not define business-level concepts such as namespace, table
   name, topic, collection name, database name, primary key, or index.
   Such concepts can be modeled as ordinary fields or carried by an
   enclosing application protocol.  The core format specified here is a
   general typed binary value container.

   The main properties of TPACK are:

   *  Self-contained messages remain the default.

   *  Cached-schema operation is supported as a profile for transports
      that establish schema state externally.

   *  Values are encoded without repeated field names or repeated type
      tags; the schema defines value order and interpretation.

   *  Decoding and validation are performed in a single pass after the
      active schema is available.

   *  Decoders can reject invalid values before handing them to
      application logic.

   *  Arbitrary-precision decimals are represented natively, not as
      JSON numbers or floating-point approximations.

   *  Temporal values support dates outside machine timestamp ranges
      and nanosecond precision.

   *  Nested structures, lists, maps, enums, unions, optional values,
      and binary data are first-class types.

   This repository also contains a Rust reference implementation and an
   executable test corpus that exercise the envelope modes, canonical
   rules, and hexadecimal examples described by this document.

# Positioning Relative to Existing Formats

   TPACK is not the first typed binary format, nor does it try to
   replace every existing one.  Its purpose is narrower: one typed
   value per message, with schema-aware validation at parse time, while
   keeping self-contained messages as the default.

   Avro is the closest prior art {{AvroSpec}}.  Like TPACK, Avro avoids
   repeating field names in every record and uses a schema to interpret
   the data.  The main difference is where schema state lives.  Avro
   object container files carry schema once per file, and Avro
   single-object encoding carries a compact schema fingerprint rather
   than the complete schema.  TPACK FullSchema instead carries the
   complete binary type descriptor inside each self-contained message.
   TPACK therefore pays more first-message overhead than Avro
   single-object encoding, but it does not require a pre-shared
   fingerprint table, JSON schema text, or an external registry in the
   core format.  TPACK SchemaRef is intentionally only a profile for
   deployments that already have such external schema state.

   Apache Arrow IPC {{ArrowFormat}} solves a different problem.  Arrow
   is optimized for columnar arrays, record batches, zero-copy
   relocation, and analytical processing.  Its primitive unit is a
   schema plus a batch-oriented body, not one nested typed value.
   TPACK does not attempt Arrow's in-memory layout guarantees or
   vectorized scan model.  Instead, TPACK targets compact transport of a
   single typed value, recursive schema trees, and immediate validation
   by a generic decoder.

   CBOR {{RFC8949}} plus CDDL {{RFC8610}} provides compact value-level
   self-description plus a separate schema language.  That combination
   is appropriate when applications want a flexible tagged value model
   and apply schema validation as an additional layer.  TPACK makes a
   different tradeoff: the active schema is part of the message or
   cached-schema profile itself, and the data block omits repeated type
   tags and field names once the schema is known.  TPACK therefore
   couples schema and value more tightly than CBOR plus CDDL, while
   giving up some of CBOR's schema-agnostic flexibility.

   Protocol Buffers, FlatBuffers, and similar IDL-driven formats remain
   good choices when deployments already depend on generated code or
   strongly controlled schemas.  TPACK version 1 deliberately avoids
   requiring generated code, code generation toolchains, or a registry
   protocol in the core wire format.

# Conventions and Terminology

   The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
   "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY",
   and "OPTIONAL" in this document are to be interpreted as described
   in BCP 14 {{RFC2119}} {{RFC8174}} when, and only when, they appear in
   all capitals, as shown here.

   The following terms are used throughout this document:

   Message:
      A complete TPACK byte sequence consisting of a header and an
      envelope.  Depending on the envelope mode, the envelope contains a
      complete schema and data block, or a SchemaId and data block.

   Schema Block:
      A binary type descriptor tree carried by FullSchema and
      FullSchemaWithId envelopes.

   SchemaId:
      An opaque byte string used by an enclosing application, stream,
      cache, or registry to identify a schema.

   Data Block:
      The encoded value whose type is described by the active schema.

   Type Descriptor:
      A binary description of one TPACK type.  Type descriptors may be
      recursive.

   Value:
      Data encoded according to a type descriptor.

   Text:
      A sequence of Unicode scalar values encoded as UTF-8.

   Byte String:
      An uninterpreted sequence of octets.

   UVarInt:
      The unsigned variable-length integer encoding defined in
      Section 5.6.

   SVarInt:
      The signed variable-length integer encoding defined in
      Section 5.7.

   Canonical Encoding:
      The unique shortest valid byte representation of a TPACK message,
      as defined in Section 10.

# Design Goals and Non-Goals

   TPACK is designed to satisfy the following goals:

   *  Stateless decoding by default.  A receiver that implements this
      specification can decode a FullSchema or FullSchemaWithId message
      without retrieving an external schema.

   *  Compact encoding.  Field names and type descriptors are encoded
      once in the schema or obtained from a schema cache.  The data
      block contains only values.

   *  Strong validation.  Type constraints are checked as values are
      parsed.

   *  Deterministic layout.  Struct values are encoded in schema order,
      maps and lists are length-prefixed, and unions are tagged by
      variant index.

   *  Language neutrality.  The format does not depend on memory layout,
      object models, generated classes, or host-language integer sizes.

   *  Generality.  The format is not tied to JSON, SQL, document
      databases, message queues, or file storage.

   *  Opaque schema identity.  SchemaId values are byte strings managed
      outside the TPACK core wire format.

   *  Implementation latitude.  Implementations may cache, pre-validate,
      or compile schemas without changing the wire format.

   The following are non-goals for TPACK version 1:

   *  TPACK is not a schema registry protocol.

   *  TPACK does not define how SchemaId values are generated.

   *  TPACK does not define schema negotiation.

   *  TPACK does not require SchemaId values to be hashes.

   *  TPACK does not require generated code, JIT compilation, or
      native-code parsers.

   *  TPACK is not a database DDL or query language.

   *  TPACK is not a stream framing protocol for multiplexing multiple
      messages over a transport.

   *  TPACK does not define compression, encryption, authentication, or
      authorization.

   *  TPACK version 1 does not define cyclic object graphs.  Directed
      graphs and cyclic references may be specified by future extensions
      using the extension mechanism in Section 7.6.

# Data Model

   A TPACK message represents exactly one typed value.  The top-level
   value MAY be any TPACK type.  Interchange formats and public APIs
   SHOULD use a top-level Struct unless there is a clear reason to use
   another type.

   The TPACK data model consists of:

   *  Null.

   *  Booleans.

   *  Fixed-width signed and unsigned integers.

   *  IEEE 754 binary floating-point numbers.

   *  Arbitrary-precision decimal numbers.

   *  UTF-8 text strings.

   *  Byte strings.

   *  Temporal values: Date, Time, DateTime, DateTimeTZ, Timestamp,
      Duration, and CalendarInterval.

   *  Arbitrary-size signed and unsigned integers.

   *  Structs with named fields.

   *  Lists.

   *  Maps.

   *  Tagged unions.

   *  Enums.

   *  Optional values.

   TPACK distinguishes absent values from null values.  A field is
   absent only when its type is Optional and the encoded presence marker
   indicates absence.  Null is an explicit value of the Null type.

   Schema authors SHOULD use Optional(T) when a field may be omitted on
   the wire.  They SHOULD use Null, or a Union that explicitly includes
   Null, only when a present value may itself be null.  Optional(T) and
   Union{Null, T} are therefore not interchangeable.  If an application
   needs three states such as absent, explicit null, and concrete
   value, one possible model is Optional(Union{Null, T}).

# Wire Format

## Overall Message Structure

   A TPACK message has the following layout:

      +--------+----------+
      | Header | Envelope |
      +--------+----------+

   The Header is fixed length.  The Envelope begins with an
   EnvelopeMode octet.  Depending on that mode, the envelope either
   carries a complete length-delimited Schema Block followed by a Data
   Block, or carries a SchemaId followed by a Data Block whose schema is
   obtained from an enclosing cached schema profile.

   A decoder MUST reject a message if bytes remain after the complete
   value has been decoded, unless an enclosing framing protocol
   explicitly carries multiple concatenated messages.  For envelopes
   that contain a complete schema, the Data Block begins immediately
   after the SchemaLen-delimited TypeDescriptor.  For SchemaRef, the
   Data Block begins immediately after the SchemaId.

   The generic grammar is:

      Message          = Header Envelope
      Header           = Magic Version
      Magic            = %x54 %x50 %x41 %x4B
      Version          = %x01

      Envelope         = FullSchema / FullSchemaWithId / SchemaRef

      FullSchema       = %x00 SchemaLen TypeDescriptor Value
      FullSchemaWithId = %x01 SchemaIdLen SchemaId
                         SchemaLen TypeDescriptor Value
      SchemaRef        = %x02 SchemaIdLen SchemaId Value

      SchemaLen        = UVarInt
      SchemaIdLen      = UVarInt
      SchemaId         = *OCTET

   The grammar above is descriptive.  The normative binary encodings of
   TypeDescriptor and Value are defined in Sections 7 and 8.
   SchemaLen is the number of octets in the complete TypeDescriptor and
   does not include the SchemaLen field itself.  The Data Block length
   is driven by the schema unless an enclosing transport also provides a
   message length.

## Header

   The TPACK version 1 header is five octets:

      54 50 41 4B 01

   The first four octets are the ASCII string "TPAK".  The fifth octet
   is the version number.  This document defines version 1 only.

   A decoder MUST reject a message whose first four octets are not
   "TPAK".  A decoder conforming only to this document MUST reject a
   message whose version octet is not 0x01.

   The format name is TPACK, but the version 1 wire magic is the
   four-octet marker "TPAK".  This document intentionally keeps that
   marker.  Changing it would create a wire-incompatible revision
   across the current draft text, examples, reference tests, and Rust
   implementation, while providing little technical benefit beyond name
   alignment.  In version 1, specifications and implementations SHOULD
   therefore use "TPACK" as the format name and "TPAK" only for the
   fixed header bytes.

## Envelope Modes

   EnvelopeMode is one octet:

      +--------+------------------+----------------------------------+
      | Value  | Name             | Meaning                          |
      +--------+------------------+----------------------------------+
      | 0x00   | FullSchema       | Complete schema and data         |
      | 0x01   | FullSchemaWithId | SchemaId, complete schema, data  |
      | 0x02   | SchemaRef        | SchemaId and data only           |
      +--------+------------------+----------------------------------+

   A decoder MUST reject an unknown EnvelopeMode.

   FullSchema is the default self-contained TPACK core message form.
   It carries SchemaLen, the complete TypeDescriptor bytes, and the
   Data Block.

   FullSchemaWithId is also self-contained.  It carries a SchemaId
   before SchemaLen and the complete TypeDescriptor.  A receiver that
   already has a trusted local binding for the SchemaId can use
   SchemaLen to skip the TypeDescriptor bytes and parse the Data Block
   using the cached schema.  A receiver without such a binding reads and
   validates the TypeDescriptor as in FullSchema.

   SchemaRef is not independently self-describing.  It is valid only in
   a cached schema profile where the receiver has already established a
   SchemaId-to-schema binding by an external mechanism.

## Schema Identifiers

   SchemaId is an opaque byte string.  TPACK does not define how
   SchemaId values are generated.  TPACK does not require SchemaId to
   be a hash, does not define a schema registry, does not define schema
   negotiation, and does not guarantee that a SchemaId is authentically
   bound to any particular schema.

   SchemaId comparison uses byte-for-byte equality.  Implementations
   MUST NOT apply Unicode normalization, case folding, text decoding, or
   other transformations when comparing SchemaId values.

   SchemaIdLen MAY be zero in FullSchemaWithId.  This is equivalent to
   carrying an empty identifier and provides no useful caching value;
   encoders SHOULD NOT emit it.  SchemaIdLen in SchemaRef MUST be
   greater than zero.  Implementations MUST enforce a configured maximum
   SchemaIdLen and reject longer identifiers.

   Applications using SchemaRef MUST establish the SchemaId-to-schema
   binding outside TPACK before the SchemaRef message is decoded.  If
   bindings are learned from an untrusted source, applications MUST use
   external protection such as authenticated transport, signatures,
   registry access control, or another mechanism appropriate to the
   deployment.  TPACK core does not verify that SchemaId and Schema
   bytes match.

   Within one cache namespace, registry authority, or other
   profile-defined binding context, a SchemaId MUST resolve to at most
   one schema.  If an implementation learns, observes, or is configured
   with multiple different schemas for the same SchemaId in that scope,
   it MUST treat that condition as a collision or configuration error.
   It MUST NOT silently overwrite one binding with another, choose a
   binding nondeterministically, or continue decoding SchemaRef against
   an ambiguous binding.

### Recommended SchemaId Profiles

   TPACK core keeps SchemaId opaque.  However, independent
   implementations often need a default convention when no registry or
   bilateral naming scheme already exists.

   In such deployments, a sender and receiver SHOULD derive SchemaId
   from the canonical TypeDescriptor bytes alone, excluding Header,
   EnvelopeMode, SchemaLen, and the Data Block.  The bytes used as hash
   input SHOULD be exactly the validated on-wire TypeDescriptor encoded
   with shortest UVarInt forms and zero field flags.

   Hashing an entire FullSchema or FullSchemaWithId message does not
   follow this convention.

   This document makes a two-layer recommendation for hash-derived
   SchemaId profiles over those canonical TypeDescriptor bytes.

   For open interoperability across independent implementations,
   uncoordinated deployments, or long-lived shared registries, SHA-256
   is RECOMMENDED as the primary default.  The resulting SchemaId is the
   bare 32-octet digest.  SHA-256 remains the main default because its
   collision resistance is appropriate for open deployments where the
   binding context may span multiple authorities, implementations, or
   long retention periods.

   This document also defines an official compact 64-bit profile named
   `xxh64-v1`.  In that profile, the SchemaId is computed as
   `xxHash64(seed=0)` over the same canonical TypeDescriptor bytes and
   serialized as a fixed 8-octet big-endian value.  The purpose of
   `xxh64-v1` is compactness and lower implementation cost in bounded
   deployments; it is not the main default for open interoperability.

   Deployments MAY use another collision-resistant hash by prior
   agreement, and MAY prepend an application-specific prefix or
   algorithm identifier if they need domain separation.

   Some deployments, including resource-constrained devices, may choose
   not to compute SHA-256 on device or may want a smaller identifier on
   the wire or in a cache key.  Such deployments MAY use `xxh64-v1`,
   registry-issued identifiers, stream-local or connection-local
   identifiers, locally assigned names, or another profile-specific
   convention.  They MAY also use another simpler or faster hash by
   prior agreement when the resulting SchemaId is only a local name.

   A profile that uses local assignment, `xxh64-v1`, or another
   non-collision-resistant or narrower-scope naming convention MUST
   define the identifier scope, the assignment authority, whether
   identifiers may be reused, and how bindings are reset after reboot,
   reconnect, firmware change, or other context loss.  In these
   profiles, SchemaId does not authenticate a schema, does not make
   collisions negligible in the open Internet sense, and is not
   suitable as the sole basis for sharing schema bindings across
   unrelated trust domains or long-lived persistent caches.  Receivers
   MUST reject SchemaRef when the required binding context is absent,
   expired, ambiguous, conflicting, or established under a different
   scope.

   This convention is informative only.  It does not change the core
   wire format, does not make SchemaId a hash by definition, and does
   not prohibit registry-issued identifiers, stream-local identifiers,
   or application-defined names encoded as bytes.

## Schema Skipping

   For FullSchemaWithId, a decoder reads SchemaIdLen and SchemaId
   before deciding how to handle the embedded schema.  If the SchemaId
   is found in a local schema cache and application policy permits
   trusting that binding, the decoder MAY skip exactly SchemaLen octets
   of TypeDescriptor bytes and parse the following Data Block with the
   cached schema.  A decoder MAY validate the skipped TypeDescriptor
   against the cached schema in the background, but this specification
   does not require it.

   A cached-schema profile MAY impose a stronger rule and require a
   decoder to compare the embedded schema bytes against the cached
   schema before the message is accepted.  Such a policy is compatible
   with this specification and can reduce cache-confusion risk.

   If a decoder validates the embedded TypeDescriptor against an
   existing binding and they differ, it MUST reject the message and
   treat the condition as a collision or configuration error in the
   active binding context.  A decoder MUST NOT silently replace the
   existing binding with the schema carried by that message.

   Even when SchemaId values follow the SHA-256 default or the
   `xxh64-v1` compact profile, that convention alone does not
   authenticate a cache namespace or registry binding.

   If a FullSchemaWithId cache lookup misses, the decoder MUST read and
   validate the SchemaLen-delimited TypeDescriptor.  If validation
   succeeds, the implementation MAY add the SchemaId-to-schema binding
   to a local cache according to application policy.  If the
   implementation already knows that the same SchemaId is conflicting or
   ambiguous in the active context, it MUST fail instead of creating or
   replacing a binding implicitly.

   For FullSchema, a decoder can use SchemaLen to locate the Data Block
   after validating the TypeDescriptor.  Without a SchemaId, a decoder
   cannot safely substitute an external schema unless an enclosing
   protocol defines equivalent binding semantics.

   For SchemaRef, a cache miss is fatal.  A decoder MUST NOT parse the
   Data Block without an established schema binding.  A decoder MUST
   also reject SchemaRef if the active profile marks the binding
   ambiguous, conflicting, expired, or out of scope for the current
   context.

   Implementations SHOULD expose errors corresponding to the following
   conditions where applicable:

      UNKNOWN_SCHEMA_ID
      INVALID_SCHEMA_ID
      SCHEMA_ID_CONFLICT
      EMBEDDED_SCHEMA_MISMATCH
      SCHEMA_REF_NOT_ALLOWED
      SCHEMA_LENGTH_EXCEEDED
      SCHEMA_LENGTH_MISMATCH

## Unsigned Variable-Length Integer

   UVarInt encodes a non-negative integer using base-128 little-endian
   continuation bytes.

   Each octet contributes seven payload bits.  The most significant bit
   is the continuation bit.  If the continuation bit is 1, another octet
   follows.  If it is 0, the integer ends.

   The numeric value is:

      value = sum((octet[i] & 0x7f) << (7 * i))

   UVarInt values MUST use the shortest possible encoding.  For
   example, zero is encoded as "00", and 128 is encoded as "80 01".
   Encoders MUST NOT emit overlong UVarInt encodings.  Decoders MUST
   reject overlong UVarInt encodings in canonical mode and SHOULD reject
   them in all modes.

   Implementations MAY impose a maximum UVarInt byte length for
   resource control.  Such limits MUST be documented.

## Signed Variable-Length Integer

   SVarInt encodes a signed integer using ZigZag transformation followed
   by UVarInt.

      zigzag(n) = 2 * n,       if n >= 0
      zigzag(n) = -2 * n - 1,  if n < 0

   The transformed value is encoded as UVarInt.  The inverse operation
   reconstructs the signed integer.

   SVarInt is unbounded at the data model level.  Implementations MAY
   impose local magnitude limits for resource control, but they MUST
   report a validation error rather than silently truncating values.

## Text String Component

   A text component is encoded as:

      Length  : UVarInt
      Content : Length octets of UTF-8

   Length is measured in octets, not Unicode scalar values and not user-
   visible characters.  Decoders MUST validate UTF-8 according to
   {{RFC3629}}.

## Byte String Component

   A byte string component is encoded as:

      Length  : UVarInt
      Content : Length octets

   Byte string content is uninterpreted by this specification.

# Schema Block

## Type Descriptor Encoding

   When an envelope carries a schema block, that schema block contains
   exactly one TypeDescriptor.  That descriptor defines the type of the
   following data block.

   Every TypeDescriptor begins with a one-octet type tag.  Some tags are
   followed by parameters.  Parameters are encoded immediately after the
   tag and are part of the schema block.

   Type descriptors are recursive.  For example, a List descriptor
   contains an element TypeDescriptor, and a Struct descriptor contains
   one TypeDescriptor for each field.

   A decoder MUST reject a schema block that is malformed, uses an
   unknown core type tag, has duplicate field identifiers or duplicate
   field names within a Struct, has duplicate symbols within an Enum, or
   contains invalid parameters.

## Type Tag Registry

   The following tags are defined by TPACK version 1:

      +--------+----------------+-----------------------------------+
      | Tag    | Name           | Meaning                           |
      +--------+----------------+-----------------------------------+
      | 0x00   | Null           | Explicit null value               |
      | 0x01   | Bool           | Boolean                           |
      | 0x02   | I8             | Signed 8-bit integer              |
      | 0x03   | I16            | Signed 16-bit integer             |
      | 0x04   | I32            | Signed 32-bit integer             |
      | 0x05   | I64            | Signed 64-bit integer             |
      | 0x06   | U8             | Unsigned 8-bit integer            |
      | 0x07   | U16            | Unsigned 16-bit integer           |
      | 0x08   | U32            | Unsigned 32-bit integer           |
      | 0x09   | U64            | Unsigned 64-bit integer           |
      | 0x0A   | F32            | IEEE 754 binary32                 |
      | 0x0B   | F64            | IEEE 754 binary64                 |
      | 0x0C   | Decimal        | Arbitrary-precision decimal       |
      | 0x0D   | Decimal(P,S)   | Decimal with precision and scale  |
      | 0x0E   | String(N)      | UTF-8 string with max byte length |
      | 0x0F   | String         | UTF-8 string                      |
      | 0x10   | Bytes(N)       | Byte string with max byte length  |
      | 0x11   | Bytes          | Byte string                       |
      | 0x12   | Date           | Proleptic Gregorian date          |
      | 0x13   | Time           | Time of day                       |
      | 0x14   | DateTime       | Date and time without timezone    |
      | 0x15   | DateTimeTZ     | Date, time, and timezone name     |
      | 0x16   | Timestamp(P)   | Epoch timestamp with precision    |
      | 0x17   | Duration       | Physical elapsed duration         |
      | 0x18   | BigInt         | Arbitrary-size signed integer     |
      | 0x19   | BigUInt        | Arbitrary-size unsigned integer   |
      | 0x1A   | CalendarInterval | Calendar-aware interval          |
      | 0x20   | Struct         | Ordered named fields              |
      | 0x21   | List           | Homogeneous sequence              |
      | 0x22   | Map            | Key-value collection              |
      | 0x23   | Union          | Tagged sum type                   |
      | 0x24   | Enum           | Named finite set                  |
      | 0x25   | Optional       | Presence-tagged value             |
      | 0x26   | Extension      | Length-delimited extension type   |
      +--------+----------------+-----------------------------------+

   Tags 0x1B through 0x1F and 0x27 through 0x7F are reserved for
   future standards-track extensions.
   Tags 0x80 through 0xEF are reserved for future use.
   Tags 0xF0 through 0xFE are private-use tags and MUST NOT appear in
   messages intended for open interchange.  Tag 0xFF is permanently
   reserved and MUST NOT be used.

## Primitive Type Descriptors

   The following tags have no schema parameters:

      Null, Bool, I8, I16, I32, I64, U8, U16, U32, U64,
      F32, F64, Decimal, String, Bytes, Date, Time, DateTime,
      DateTimeTZ, Duration, BigInt, BigUInt, CalendarInterval.

   Their TypeDescriptor is exactly one octet: the type tag.

## Parameterized Type Descriptors

### Decimal(P,S)

   Decimal(P,S) is encoded as:

      Tag        : 0x0D
      Precision  : UVarInt
      Scale      : UVarInt

   Precision is the maximum number of decimal digits in the absolute
   value of the coefficient.  Precision MUST be greater than zero.

   Scale is the fixed number of fractional decimal digits.  Scale MUST
   be less than or equal to Precision.

   A Decimal(P,S) value is interpreted as:

      value = coefficient * 10^(-Scale)

   The coefficient is encoded in the data block as SVarInt.  The scale
   is not repeated in the data block.

### String(N)

   String(N) is encoded as:

      Tag          : 0x0E
      MaxLength    : UVarInt

   MaxLength is the maximum allowed number of UTF-8 octets in the value.
   MaxLength MAY be zero.  The name String(N) denotes a bounded string,
   not a string that is always exactly N octets long.

### Bytes(N)

   Bytes(N) is encoded as:

      Tag          : 0x10
      MaxLength    : UVarInt

   MaxLength is the maximum allowed number of octets in the value.
   MaxLength MAY be zero.  The name Bytes(N) denotes a bounded byte
   string, not a byte string that is always exactly N octets long.

### Timestamp(P)

   Timestamp(P) is encoded as:

      Tag          : 0x16
      Precision    : one octet

   Precision MUST be one of:

      0x00  seconds
      0x01  milliseconds
      0x02  microseconds
      0x03  nanoseconds

   A Timestamp(P) value is an SVarInt count of units since
   1970-01-01T00:00:00Z on the POSIX time scale.  Leap seconds are not
   represented by Timestamp(P).  Applications that require civil-time
   representation with a timezone name SHOULD use DateTimeTZ.

## Structural Type Descriptors

### Struct

   Struct is encoded as:

      Tag          : 0x20
      FieldCount   : UVarInt
      Field[0..n)  : FieldDescriptor

   Each FieldDescriptor is encoded as:

      FieldId      : UVarInt
      Name         : Text component
      Flags        : UVarInt
      Type         : TypeDescriptor

   FieldId MUST be greater than zero and MUST be unique within the
   enclosing Struct.  Once a FieldId has been published for a field, it
   MUST NOT be reused for a different semantic field within the same
   application schema.  If a field is renamed, its FieldId SHOULD stay
   the same.  If a field is removed, its FieldId SHOULD be considered
   retired.

   Field names MUST be valid UTF-8, MUST be non-empty, and MUST be
   unique within the enclosing Struct.  Field names are descriptive and
   human-readable; FieldId is the stronger compatibility key for
   application model binding.

   Field order is significant for the wire format.  Struct values are
   encoded in exactly this order.  A generic decoder decodes values in
   schema order and MAY then build an object mapping by FieldId and
   name for application use.

   Flags is reserved for future use in version 1.  Encoders MUST write
   zero.  Decoders conforming to this document MUST reject non-zero
   field flags.

   Optionality is represented by wrapping a field type in Optional, not
   by omitting the field descriptor.

### List

   List is encoded as:

      Tag          : 0x21
      MaxCount     : UVarInt
      ElementType  : TypeDescriptor

   MaxCount equal to zero means unbounded by schema.  MaxCount greater
   than zero means that list values MUST contain no more than MaxCount
   elements.

### Map

   Map is encoded as:

      Tag          : 0x22
      MaxCount     : UVarInt
      KeyType      : TypeDescriptor
      ValueType    : TypeDescriptor

   MaxCount equal to zero means unbounded by schema.  MaxCount greater
   than zero means that map values MUST contain no more than MaxCount
   entries.

   Map keys MUST NOT be Null, Optional, List, Map, Struct, Union, or
   Extension in TPACK version 1.  Enum keys are permitted.  Floating-
   point keys are permitted only if the encoded value is not NaN.

   A decoder that exposes a Map abstraction MUST reject duplicate keys.
   Key equality is determined by the canonical encoded key bytes.
   Decoders MAY offer a streaming entries API that does not check
   duplicates immediately, but such an API MUST NOT expose the result as
   a validated Map until uniqueness has been verified.

### Union

   Union is encoded as:

      Tag            : 0x23
      VariantCount   : UVarInt
      Variant[0..n)  : VariantDescriptor

   Each VariantDescriptor is encoded as:

      Name           : Text component
      Type           : TypeDescriptor

   Variant names MUST be valid UTF-8, MUST be non-empty, and MUST be
   unique within the enclosing Union.

### Enum

   Enum is encoded as:

      Tag            : 0x24
      SymbolCount    : UVarInt
      Symbol[0..n)   : Text component

   Symbol names MUST be valid UTF-8, MUST be non-empty, and MUST be
   unique within the enclosing Enum.

### Optional

   Optional is encoded as:

      Tag            : 0x25
      InnerType      : TypeDescriptor

   Optional(Null) is valid but not useful.  Encoders SHOULD NOT emit
   Optional(Null).

## Extension Type Descriptor

   Extension is encoded as:

      Tag            : 0x26
      Authority      : Text component
      TypeName       : Text component
      SchemaParams   : Byte string component

   The Authority field SHOULD be a domain name, URI, or other stable
   namespace controlled by the extension author.  TypeName identifies
   the extension type within that authority.  SchemaParams is an
   extension-defined byte string.

   Values of an Extension type are encoded as a byte string component.
   This makes unknown extension values skippable by generic decoders.
   A generic decoder that does not understand an Extension type MUST
   treat the value as opaque bytes and MUST NOT claim semantic
   validation beyond length correctness.

   Extension types intended for interoperable use SHOULD be documented
   in a public specification.  Private-use one-byte type tags 0xF0
   through 0xFE are available for closed systems, but they are not
   self-describing to generic decoders and are therefore unsuitable for
   open interchange.

# Data Block

## General Rule

   The data block contains exactly one Value encoded according to the
   active TypeDescriptor.  In FullSchema and FullSchemaWithId, the
   active TypeDescriptor is carried in the message.  In SchemaRef, it is
   obtained from the cached schema profile.  Values do not repeat their
   type tags.  The schema supplies all type information except for
   Union variant indexes and Optional presence markers.

   A decoder MUST reject a data block that does not contain enough bytes
   for the described value.  A decoder MUST also reject a data block
   that contains trailing bytes after the described value.

## Primitive Values

   Null:
      Encoded as zero octets.

   Bool:
      Encoded as one octet.  False is 0x00.  True is 0x01.  All other
      values MUST be rejected.

   I8:
      Encoded as one octet, two's complement.

   I16:
      Encoded as two octets, big-endian, two's complement.

   I32:
      Encoded as four octets, big-endian, two's complement.

   I64:
      Encoded as eight octets, big-endian, two's complement.

   U8:
      Encoded as one octet.

   U16:
      Encoded as two octets, big-endian.

   U32:
      Encoded as four octets, big-endian.

   U64:
      Encoded as eight octets, big-endian.

   F32:
      Encoded as four octets containing an IEEE 754 binary32 value in
      big-endian order.

   F64:
      Encoded as eight octets containing an IEEE 754 binary64 value in
      big-endian order.

## Decimal Values

   Decimal is the unconstrained arbitrary-precision decimal type.
   Decimal values are encoded as:

      Scale        : SVarInt
      Coefficient  : SVarInt

   The numeric value is:

      value = Coefficient * 10^(-Scale)

   Scale MAY be negative.  For example, Scale -3 means the coefficient
   is multiplied by 1000.

   Decimal(P,S) values are encoded as:

      Coefficient  : SVarInt

   The numeric value is:

      value = Coefficient * 10^(-S)

   A Decimal(P,S) value is valid only if the absolute coefficient has no
   more than P decimal digits.  The value zero has precision 1 for this
   validation rule.

   Encoders SHOULD choose the shortest decimal representation that
   preserves the schema-defined scale.  Encoders MUST NOT encode decimal
   values as IEEE binary floating-point values.

## Text and Byte Values

   String values are encoded as a text component.  String(N) values are
   encoded identically, then validated to ensure that the byte length is
   less than or equal to N.

   Bytes values are encoded as a byte string component.  Bytes(N) values
   are encoded identically, then validated to ensure that the byte
   length is less than or equal to N.

## Temporal Values

   TPACK uses the proleptic Gregorian calendar for Date and DateTime
   values.  Year zero is permitted and follows astronomical year
   numbering: year 0 is 1 BCE, year -1 is 2 BCE, and so on.

   Date:
      Encoded as SVarInt days relative to 1970-01-01.  Day 0 is
      1970-01-01.  Negative values represent earlier dates.

   Time:
      Encoded as UVarInt nanoseconds since local midnight.  The value
      MUST be less than 86,400,000,000,000.

   DateTime:
      Encoded as Date followed by Time:

         DaysSinceEpoch : SVarInt
         NanosOfDay     : UVarInt

      DateTime has no timezone.  It represents a civil date and local
      time, not an instant on the UTC timeline.

   DateTimeTZ:
      Encoded as DateTime followed by a timezone identifier:

         DaysSinceEpoch : SVarInt
         NanosOfDay     : UVarInt
         TimeZone       : Text component

      TimeZone SHOULD be an IANA time zone database name such as
      "UTC" or "Asia/Shanghai".  This specification does not embed
      timezone database rules.  Applications MUST define which timezone
      database version they use when exact historical interpretation is
      required.

   Timestamp(P):
      Encoded as SVarInt units since 1970-01-01T00:00:00Z.  The unit is
      determined by the Precision parameter in Section 7.4.4.

   Duration:
      Encoded as two SVarInt values:

         Seconds : SVarInt
         Nanos   : SVarInt

      Nanos is a signed nanosecond adjustment whose absolute value MUST
      be less than 1,000,000,000.  Seconds and Nanos MUST have the same
      sign unless either value is zero.  Duration represents a physical
      elapsed duration and is suitable for timeouts, latency, TTLs, and
      similar measurements.

   CalendarInterval:
      Encoded as three SVarInt values:

         Months : SVarInt
         Days   : SVarInt
         Nanos  : SVarInt

      Months and days are separate because calendar months are not a
      fixed number of days.  CalendarInterval represents calendar
      semantics such as "1 month 3 days".  SDKs MUST NOT automatically
      normalize CalendarInterval values to Duration.  SDKs MUST NOT
      apply a CalendarInterval to a Timestamp without explicit calendar
      and timezone context.

## Big Integer Values

   BigInt is encoded as SVarInt.

   BigUInt is encoded as UVarInt.

   These types are arbitrary-size at the data model level.  A decoder
   MAY impose implementation-specific limits and reject values that
   exceed those limits.

## Structural Values

### Struct Values

   A Struct value is encoded as the concatenation of its field values in
   schema order.  Field names are not repeated in the data block.

   If a field type is Optional, the Optional encoding determines whether
   the field value is present.  Otherwise the field value MUST be
   encoded.

### List Values

   A List value is encoded as:

      Count        : UVarInt
      Element[0..n): Value

   Count is the number of elements.  Each element is encoded according
   to the ElementType in the List type descriptor.  If MaxCount is
   non-zero, Count MUST be less than or equal to MaxCount.

### Map Values

   A Map value is encoded as:

      Count          : UVarInt
      Entry[0..n)    : Key Value

   Each key is encoded according to KeyType.  Each value is encoded
   according to ValueType.  If MaxCount is non-zero, Count MUST be less
   than or equal to MaxCount.

   Outside canonical mode, map entries MAY appear in any order.  A
   decoder that exposes a Map abstraction MUST reject duplicate keys.
   A decoder MAY offer a streaming entries API that does not check
   duplicates immediately, but then it MUST NOT expose the result as a
   validated Map until uniqueness is verified.

   In canonical mode, entries MUST be sorted by ascending lexicographic
   order of the canonical encoded key bytes, and duplicate keys MUST be
   rejected.  A decoder operating in canonical mode MAY check sortedness
   incrementally by comparing each key with the previous key.

   For high-throughput streaming data where key uniqueness is not
   needed, applications SHOULD use List<Struct { key, value }> rather
   than Map.

### Union Values

   A Union value is encoded as:

      VariantIndex  : UVarInt
      VariantValue  : Value

   VariantIndex is zero-based and selects one of the variants in schema
   order.  VariantIndex MUST be less than VariantCount.  VariantValue is
   encoded according to the selected variant's TypeDescriptor.

### Enum Values

   An Enum value is encoded as:

      SymbolIndex   : UVarInt

   SymbolIndex is zero-based and MUST be less than SymbolCount.

### Optional Values

   An Optional value is encoded as:

      Presence      : one octet
      Value         : present only if Presence is 0x01

   Presence 0x00 means absent.  Presence 0x01 means present.  All other
   Presence values MUST be rejected.

# Validation Rules

   A conforming decoder MUST validate the following while parsing:

   *  Header magic and version.

   *  EnvelopeMode is known.

   *  SchemaLen matches the length of a complete parseable
      TypeDescriptor.

   *  SchemaIdLen does not exceed the implementation's configured
      maximum.

   *  SchemaRef carries a non-empty SchemaId.

   *  SchemaRef is allowed by the active profile and has a cache hit
      before the Data Block is parsed.

   *  UVarInt and SVarInt shortest form when operating in canonical
      mode.

   *  UTF-8 validity for all text components.

   *  Uniqueness of Struct field identifiers and field names, Union
      variant names, and Enum symbols.

   *  Struct FieldId values are greater than zero.

   *  Reserved field flags are zero.

   *  Decimal(P,S) has Precision > 0 and Scale <= Precision.

   *  Decimal(P,S) values do not exceed declared precision.

   *  String(N) and Bytes(N) values do not exceed declared byte length.

   *  Time values are less than 86,400,000,000,000 nanoseconds.

   *  Timestamp(P) precision values are known.

   *  Duration Nanos has absolute value less than 1,000,000,000, and
      Duration Seconds and Nanos have the same sign unless either value
      is zero.

   *  CalendarInterval is preserved as calendar semantics and is not
      normalized to Duration by the decoder or SDK.

   *  List and Map counts do not exceed MaxCount if MaxCount is nonzero.

   *  Union and Enum indexes are in range.

   *  Optional presence markers are either 0x00 or 0x01.

   *  Map keys are unique before the value is exposed as a validated Map.

   *  Canonical Map ordering is enforced when operating in canonical
      mode.

   *  No trailing bytes remain after the data block value is decoded.

   Decoders SHOULD expose errors that identify the failing path within
   the schema, for example "/price" or "/items/3/amount".  Error path
   reporting is not part of the wire format.

# Canonical Encoding

   Canonical TPACK is the unique shortest valid byte representation of
   a value and schema.  Canonical encoding is REQUIRED when TPACK is
   used for hashing, signing, content addressing, or deterministic
   comparison.

   A canonical encoder MUST:

   *  Use the exact header defined in Section 5.2.

   *  Use the envelope mode declared by the application protocol.  For
      standalone interchange, use FullSchema.  If an application
      protocol includes SchemaId in material that is hashed, signed, or
      compared, it MUST explicitly declare which envelope modes are
      canonical for that protocol.

   *  Include SchemaId as opaque bytes in the canonical byte sequence
      whenever the selected envelope mode carries a SchemaId.

   *  Use the shortest UVarInt and SVarInt encodings.

   *  Encode SchemaLen and FieldId using the shortest UVarInt encoding.

   *  Encode integers in the fixed-width byte lengths defined by their
      types.

   *  Encode UTF-8 text without invalid or non-shortest UTF-8 forms.

   *  Encode Bool as exactly 0x00 or 0x01.

   *  Encode Optional presence as exactly 0x00 or 0x01.

   *  Encode F32 NaN, if present, as 7F C0 00 00.

   *  Encode F64 NaN, if present, as 7F F8 00 00 00 00 00 00.

   *  Sort Map entries by ascending lexicographic order of the canonical
      encoded key bytes.

   *  Use zero field flags.

   TPACK does not impose a canonical order on Struct fields beyond the
   order chosen by the schema.  Struct fields are not automatically
   sorted by FieldId.  Two schemas with the same field identifiers and
   names in different orders are distinct wire schemas, although their
   application binding can still be equivalent if the FieldIds are
   stable.

   TPACK does not define a canonical SchemaId generation method and
   does not require SchemaId to equal any hash of the schema.

# Schema Evolution and Unknown Data

   TPACK wire encoding for Struct values is schema-order positional.
   A generic decoder decodes fields in schema order.  Application model
   binding SHOULD then use FieldId when available, and use Name as a
   secondary descriptive key.  Name is human-readable metadata and is
   not the strongest compatibility key.

   Because FullSchema and FullSchemaWithId carry a complete schema, a
   receiver can inspect the incoming schema and decide whether it can
   process the value.  SchemaRef requires the same decision to be made
   against a schema already established by the cached schema profile.

   A receiver MAY ignore fields it does not understand if it can safely
   skip their values.  Since the schema is complete or already cached,
   all core TPACK values are skippable by a conforming decoder.
   Extension values are also skippable because their data encoding is
   length-delimited.

   Adding fields is compatible only when receivers can skip unknown
   FieldId values.  Removing fields requires receivers to tolerate
   absence if the target model permits it.  Renaming a field does not
   change semantic binding when the FieldId remains stable.  Reordering
   fields changes schema bytes and value order, but it need not change
   application binding if FieldIds are stable and the receiver decodes
   according to the transmitted schema.

   Changing the type or meaning of an existing FieldId is incompatible
   unless compatibility is explicitly modeled with Union, Optional, or a
   new FieldId.  Once a FieldId has represented one semantic field, it
   SHOULD NOT be reused for a different semantic field.

   Applications that require forward compatibility SHOULD model unknown
   data as ignored fields, Optional fields, Union variants, or Extension
   values rather than by changing primitive meanings.

# Cached Schema Profile and Framing

   TPACK defines the encoding of one message.  It does not define how
   multiple messages are framed on a byte stream.

   A transport protocol MAY frame each TPACK message using a length
   prefix, record boundary, datagram boundary, or another mechanism.
   When TPACK messages are concatenated without an external length
   prefix, a receiver can parse one message by reading the envelope,
   schema information, and data value, but error recovery after
   malformed input is transport specific.

   FullSchema is appropriate for single messages, files, offline
   storage, stateless exchange, and cross-system interchange.

   FullSchemaWithId is appropriate for warm-up, first send, and cache
   population.  It remains self-contained while allowing receivers that
   already know the SchemaId to skip the embedded schema bytes.

   SchemaRef is appropriate for long-lived connections, high-frequency
   telemetry, game state synchronization, IoT streams, and bulk streams
   where schema state has already been established.  SchemaRef is not a
   standalone self-describing message.

   A cached schema profile MUST define:

   *  Schema cache lifetime.

   *  Schema cache capacity.

   *  Schema eviction behavior.

   *  Unknown SchemaId behavior.

   *  SchemaId namespace scope and assignment authority.

   *  Collision handling, including whether conflicting bindings are a
      fatal configuration error.

   *  Whether schemas may be reused across connections.

   *  Whether authentication is required.

   *  Whether locally assigned or profile-local identifiers may be
      reused after reboot, reconnect, or firmware update.

   *  Whether SchemaRef is allowed before a schema has been established
      in the current context.

   This document defines the envelope format and generic semantics.  It
   does not define a cache management protocol.

# Implementation Status and Conformance Boundary

   This section is informative.  It records the implementation state of
   the repository that accompanies this draft and is intended to help
   reviewers separate the wire specification from the current execution
   boundary of the reference code.

   The repository contains a Rust reference implementation centered on
   `tpack-core`, plus integration crates and command-line tooling.  At
   the time of writing, the Rust implementation covers:

   *  The three envelope modes FullSchema, FullSchemaWithId, and
      SchemaRef.

   *  Schema validation for duplicate field identifiers and names,
      duplicate enum symbols, duplicate union variant names, reserved
      field flags, type-parameter checks, and configured resource
      limits.

   *  Canonical-mode checks for shortest varints, canonical NaN
      encodings, canonical Map key ordering, and duplicate Map keys
      using canonical encoded key bytes.

   *  Validation of the hexadecimal examples in this document through
      executable tests in `crates/tpack/tests/reference.rs`.

   The current Rust implementation also performs an implementation-
   specific safety check for FullSchemaWithId cache hits by default: if
   a SchemaId already exists in the local registry, the decoder reparses
   the embedded schema block and rejects the message if the embedded
   schema does not equal the cached schema.  Callers can disable that
   comparison by local policy, but only when the SchemaId namespace and
   binding source are already trusted out of band.  The stricter
   behavior is the default in the reference code.

   The specification data model defines Decimal, BigInt, and BigUInt as
   arbitrary-precision types.  The current Rust implementation does not
   yet expose arbitrary-precision semantics at that boundary.  Its
   public value model currently uses signed or unsigned 64-bit host
   integers for:

   *  Decimal scale and coefficient.

   *  Decimal(P,S) coefficient.

   *  BigInt values.

   *  BigUInt values.

   Consequently, the present Rust implementation is a conforming
   reference for TPACK version 1 only for the subset of messages whose
   values fit within those 64-bit ranges.  Inputs that require larger
   magnitudes are outside the current reference boundary and are
   rejected during varint decoding or cannot be represented by the
   implementation's exposed value API.  This limitation does not imply
   any wire-format change; a future implementation can support larger
   magnitudes while remaining wire compatible.

   Reviewers should therefore treat the Rust codebase as strong evidence
   for the envelope layout, schema encoding, validation rules,
   canonicalization behavior, and cached-schema semantics, but not yet
   as a complete executable oracle for every arbitrary-precision case
   allowed by the abstract data model.

# Test Vectors

   The hexadecimal examples in this document are intended to be
   executable test vectors, not merely illustrative prose.

   The accompanying Rust test suite verifies at least the following:

   *  The FullSchema, FullSchemaWithId, and SchemaRef encodings in the
      Examples section.

   *  Canonical Map ordering and duplicate-key rejection.

   *  Rejection of non-zero reserved field flags, trailing bytes,
      overlong varints, and non-canonical NaN encodings.

   *  Round-trip coverage for temporal values, extension values,
      Optional values, and representative nested structures.

   Future implementations SHOULD use these examples as an initial
   interoperability corpus and SHOULD add additional vectors for large
   arbitrary-precision numbers once such implementations are available.

# Examples

## Flat Record with Decimal Values

   This example encodes the following typed record:

      Struct {
        1: id    String(64)
        2: price Decimal(18,4)
        3: tax   Decimal
        4: qty   I32
        5: ts    I64
      }

   with the following value:

      {
        id:    "prod_001",
        price: 299.9900,
        tax:   13.725,
        qty:   10,
        ts:    1715000000
      }

   In this example, Decimal(18,4) fixes the scale at 4.  Therefore the
   price coefficient is 2999900.  The unconstrained Decimal tax value
   carries its own scale 3 and coefficient 13725.

   Hexadecimal encoding:

      54 50 41 4B 01

      00 28
      20 05
         01 02 69 64 00 0E 40
         02 05 70 72 69 63 65 00 0D 12 04
         03 03 74 61 78 00 0C
         04 03 71 74 79 00 04
         05 02 74 73 00 05

      08 70 72 6F 64 5F 30 30 31
      B8 99 EE 02
      06 BA D6 01
      00 00 00 0A
      00 00 00 00 66 38 D2 C0

   The first line is the header.  The second line is EnvelopeMode 0x00
   (FullSchema) followed by SchemaLen 0x28.  The schema begins with
   Struct tag 0x20 and field count 5.  Each field descriptor is
   FieldId, name, flags, and type.  The data block then stores only the
   five field values in schema order.

## Nested Structure

   The following schema contains a nested address object:

      Struct {
        1: id String(32)
        2: address Struct {
          1: city   String(64)
          2: street String(128)
          3: zip    String(16)
        }
      }

   Its schema descriptor is:

      20 02
         01 02 69 64 00 0E 20
         02 07 61 64 64 72 65 73 73 00
            20 03
               01 04 63 69 74 79 00 0E 40
               02 06 73 74 72 65 65 74 00 0E 80 01
               03 03 7A 69 70 00 0E 10

   The nested Struct is encoded recursively inside the schema.  In the
   data block, the nested Struct value is encoded as the concatenation
   of city, street, and zip values.  It does not repeat field names.

## Union Value

   The following schema models a payment amount that can be one of
   three representations:

      Union {
        fiat:   Decimal(18,4),
        label:  String,
        wei:    BigUInt
      }

   The schema descriptor is:

      23 03
         04 66 69 61 74 0D 12 04
         05 6C 61 62 65 6C 0F
         03 77 65 69 19

   As a FullSchema message, the header and envelope prefix are:

      54 50 41 4B 01 00 16

   A value selecting the "fiat" variant with amount 128.5000 is encoded
   as:

      00 90 EE 9C 01

   The first octet is VariantIndex 0.  The remaining bytes are the
   SVarInt coefficient 1285000 for Decimal(18,4).

## FullSchemaWithId Envelope

   The flat record from Section 15.1 can also be sent as
   FullSchemaWithId with SchemaId "example.record.v1":

      54 50 41 4B 01

      01
      11 65 78 61 6D 70 6C 65 2E 72 65 63 6F 72 64 2E 76 31
      28

      20 05
         01 02 69 64 00 0E 40
         02 05 70 72 69 63 65 00 0D 12 04
         03 03 74 61 78 00 0C
         04 03 71 74 79 00 04
         05 02 74 73 00 05

      08 70 72 6F 64 5F 30 30 31
      B8 99 EE 02
      06 BA D6 01
      00 00 00 0A
      00 00 00 00 66 38 D2 C0

   EnvelopeMode is 0x01.  SchemaIdLen is 0x11, followed by the opaque
   SchemaId bytes and SchemaLen 0x28.  If the receiver already trusts a
   cached binding for that SchemaId, it can skip the 0x28 schema bytes
   and parse the data with the cached schema.

## SchemaRef Envelope

   After a cached schema profile has established the same
   "example.record.v1" binding, the same data block can be sent without
   the schema bytes:

      54 50 41 4B 01

      02
      11 65 78 61 6D 70 6C 65 2E 72 65 63 6F 72 64 2E 76 31

      08 70 72 6F 64 5F 30 30 31
      B8 99 EE 02
      06 BA D6 01
      00 00 00 0A
      00 00 00 00 66 38 D2 C0

   EnvelopeMode is 0x02.  A decoder that does not already have a
   trusted binding for the SchemaId MUST reject this message.

# IANA Considerations

   This document has no IANA actions.

   If TPACK is standardized in the future, a registry for TPACK core
   type tags, extension authorities, and media types may be requested.
   A provisional media type such as application/tpack may be used only
   where permitted by local policy; this document does not register it.

# Security Considerations

   TPACK is a binary format and must be parsed defensively.

   Implementations MUST enforce resource limits.  At minimum, decoders
   SHOULD provide configurable limits for:

   *  Total message size.

   *  Schema depth.

   *  Struct field count.

   *  List and Map element count.

   *  Text and byte string length.

   *  Decimal and BigInt magnitude.

   *  Extension payload size.

   *  SchemaId length and schema cache size.

   *  Time spent validating map-key uniqueness.

   Implementations MUST NOT allocate memory solely based on an untrusted
   length prefix without checking configured limits.

   Decoders MUST validate UTF-8 before exposing strings to application
   logic.  Applications that compare or display text SHOULD consider
   Unicode normalization and confusable characters.  TPACK validates
   UTF-8 syntax but does not normalize text.

   Decimal and BigInt values can be extremely large.  Arithmetic on such
   values may be expensive even if parsing is successful.  Applications
   SHOULD validate business-level magnitude and precision constraints in
   addition to schema-level constraints.

   Extension values are opaque to generic decoders.  Applications MUST
   NOT treat an unknown extension as semantically validated merely
   because it is length-delimited and syntactically well-formed.

   SchemaId values are opaque identifiers, not proof that a schema is
   authentic or correct.  Applications that accept SchemaRef or skip an
   embedded schema in FullSchemaWithId MUST ensure that the
   SchemaId-to-schema binding was established by a trusted external
   mechanism.  If bindings are distributed over an untrusted channel,
   applications MUST protect them with authenticated transport,
   signatures, registry authorization, or equivalent controls.

   If a deployment uses hash-derived SchemaId values, the safety of that
   convention depends on the collision resistance of the chosen hash and
   on clear documentation of the hash input.  Deployments SHOULD use a
   collision-resistant hash such as SHA-256 and SHOULD scope any local
   cache or registry to the correct tenant, authority, stream, or trust
   domain.

   Deployments that intentionally use the compact `xxh64-v1` profile, a
   simpler or faster hash, or a locally assigned SchemaId, for
   constrained devices MUST scope that convention to a closed profile
   such as a single authenticated connection, stream, boot session, or
   authoritative registry.  Such identifiers MUST NOT be treated as
   portable proof that two different deployments, caches, or
   administrative domains mean the same schema.  SchemaRef MUST be
   rejected once that profile-specific binding context is lost,
   expired, ambiguous, or conflicting.

   If two different schemas are observed for the same SchemaId within
   one binding context, the receiver MUST treat that as a collision or
   configuration error.  SchemaRef cannot be decoded safely in that
   state, and FullSchemaWithId MUST NOT silently overwrite the existing
   binding.

   A malicious sender can use a known SchemaId with data encoded for a
   different schema if the application accepts unauthenticated bindings
   or confuses cache namespaces.  Cached schema profiles SHOULD scope
   caches by connection, tenant, authority, or another deployment-
   appropriate boundary and SHOULD define explicit eviction behavior.

   TPACK does not provide confidentiality, integrity, replay protection,
   or authentication.  Applications requiring these properties MUST use
   external mechanisms such as TLS, object signatures, MACs, or an
   authenticated container format.

--- back

# Rationale

   TPACK separates schema from data inside the envelope.  In the core
   FullSchema form, the schema remains inside the same message as the
   data.  Repeating a type tag for every value makes data
   self-describing but wastes space in structured records.  Requiring an
   external schema reduces payload size but makes the receiver stateful.
   TPACK therefore keeps self-contained messages as the default and adds
   SchemaRef only as an explicit cached schema profile.

   SchemaId is opaque because different deployments need different
   identity, discovery, and trust models.  Some applications may use a
   content hash, while others may use a registry key, stream-local
   integer encoded as bytes, or a protocol-specific name.  Those choices
   affect cache management and trust but not the TPACK wire format.

   Decimal is encoded as a base-10 coefficient and scale rather than a
   binary floating-point value.  This preserves exact decimal values and
   avoids JSON number precision issues.  Decimal(P,S) exists for systems
   that need database-like precision and scale constraints.  Decimal
   without parameters exists for arbitrary-precision interchange.

   Date and DateTime use variable-length integer day counts rather than
   fixed-width Unix microseconds.  This allows very old and very future
   civil dates without changing the wire format.  Timestamp(P) is kept
   as a separate type for applications that need epoch-based instants.
   Duration and CalendarInterval are separate because elapsed physical
   time and calendar arithmetic have different semantics.

   Struct FieldId preserves stable semantic binding while the data block
   remains compact and positional.  Field names remain useful for human
   understanding, diagnostics, and fallback binding, but renames should
   not by themselves change application semantics.  Struct field flags
   are reserved even though version 1 requires zero.  Reserving the slot
   makes later additions possible without changing the basic field
   descriptor shape.

   The version 1 wire magic remains "TPAK" even though the format name
   is TPACK.  This is a conscious compatibility choice for the current
   revision line, not an accidental inconsistency left unexplained.

   TPACK intentionally omits namespace and table-name fields from the
   core format.  Those concepts belong to application protocols.  A
   database ingestion protocol can wrap TPACK or include ordinary
   fields such as "namespace" and "table" in a Struct if it needs them.

# Implementation Notes

   Implementations MAY compile validated schemas into reusable parse
   plans.  Such parse plans may include field offsets, validation
   closures, specialized decoders, generated code, or native code.
   These techniques do not affect the wire format and are outside the
   scope of this specification.

   The specification does not define a JIT assembly format, internal
   intermediate representation, code generation ABI, cache-key
   algorithm, or CPU-architecture-specific behavior.

# Acknowledgements

   This draft was derived from design discussions about self-contained
   typed binary messages, parse-time validation, decimal representation,
   temporal types, and schema-carrying payloads.
