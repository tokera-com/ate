/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.dto.msg;

import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import com.google.gson.annotations.Expose;
import com.tokera.ate.annotations.YamlTag;

import javax.validation.constraints.NotNull;
import java.io.Serializable;

/**
 * Represents metadata about a data message that was placed on the distributed commit log
 */
@YamlTag("msg.meta")
public class MessageMetaDto implements Serializable {

    private static final long serialVersionUID = -1978186226449951313L;

    @Expose
    @JsonProperty
    @NotNull
    private long partition;
    @Expose
    @JsonProperty
    @NotNull
    private long offset;
    @Expose
    @JsonProperty
    @NotNull
    private long timestamp;

    @JsonIgnore
    private transient boolean _immutable = false;

    @SuppressWarnings("initialization.fields.uninitialized")
    @Deprecated
    public MessageMetaDto() {
    }

    public MessageMetaDto(long partition, long offset, long timestamp) {
        this.partition = partition;
        this.offset = offset;
        this.timestamp = timestamp;
    }
    
    public long getPartition() {
        return partition;
    }

    public void setPartition(long partition) {
        assert this._immutable == false;
        this.partition = partition;
    }

    public long getOffset() {
        return offset;
    }

    public void setOffset(long offset){
        assert this._immutable == false;
        this.offset = offset;
    }

    public long getTimestamp() {
        return timestamp;
    }

    public void setTimestamp(long timestamp) {
        assert this._immutable == false;
        this.timestamp = timestamp;
    }

    public void immutalize() {
        this._immutable = true;
    }
}
