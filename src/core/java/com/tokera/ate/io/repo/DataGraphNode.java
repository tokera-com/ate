package com.tokera.ate.io.repo;

import com.tokera.ate.dto.msg.MessageDataMetaDto;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.*;
public class DataGraphNode {

    public final MessageDataMetaDto     msg;
    public final UUID                   version;
    public final @Nullable UUID         previousVersion;
    public @Nullable DataGraphNode      parentNode;
    public LinkedList<DataGraphNode>    children = new LinkedList<>();
    public final Set<UUID>              mergesVersions;

    public DataGraphNode(MessageDataMetaDto msg) {
        this.msg = msg;
        this.version = msg.version();
        this.previousVersion =  msg.getData().getHeader().getPreviousVersion();
        this.mergesVersions = msg.getData().getHeader().getMerges();
    }

    public void attachHere(DataGraphNode node) {
        if (this.children.contains(node)) {
            return;
        }
        this.children.addLast(node);
        this.parentNode = this;
    }
}