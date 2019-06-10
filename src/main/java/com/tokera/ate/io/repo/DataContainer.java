/*
 * To change this license header, choose License Headers in Project Properties.
 * To change this template file, choose Tools | Templates
 * and open the template in the editor.
 */
package com.tokera.ate.io.repo;

import com.tokera.ate.common.MapTools;
import com.tokera.ate.dao.base.BaseDao;
import com.tokera.ate.delegates.AteDelegate;
import com.tokera.ate.dto.EffectivePermissions;
import com.tokera.ate.dto.msg.*;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.io.merge.MergePair;
import org.checkerframework.checker.nullness.qual.NonNull;
import org.checkerframework.checker.nullness.qual.Nullable;

import java.util.*;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantReadWriteLock;
import java.util.stream.Collectors;

public class DataContainer {
    public final IPartitionKey partitionKey;
    public final Map<UUID, @NonNull DataGraphNode> lookup = new HashMap<>();
    public final LinkedList<DataGraphNode> timeline = new LinkedList<>();
    public final LinkedList<DataGraphNode> leaves = new LinkedList<>();
    private final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();

    public DataContainer(IPartitionKey partitionKey) {
        this.partitionKey = partitionKey;
    }

    private DataContainer add(MessageDataMetaDto msg) {
        DataGraphNode node = new DataGraphNode(msg);
        Lock w = this.lock.writeLock();
        w.lock();
        try {
            DataGraphNode previous = MapTools.getOrNull(lookup, node.previousVersion);
            if (previous != null) {
                previous.attachHere(node);
                leaves.remove(previous);
            }
            for (UUID mergesVersion : node.mergesVersions) {
                DataGraphNode merges = MapTools.getOrNull(lookup, mergesVersion);
                if (merges == null) continue;
                leaves.remove(merges);
            }
            lookup.put(node.version, node);
            leaves.addLast(node);
            timeline.addLast(node);
            msg.immutalize();
        } finally {
            w.unlock();
        }
        return this;
    }

    public DataContainer add(MessageDataDto data, MessageMetaDto meta) {
        MessageDataMetaDto msg = new MessageDataMetaDto(data, meta);
        this.add(msg);
        return this;
    }

    public @Nullable MessageDataMetaDto getLastOrNull() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            if (timeline.size() <= 0) return null;
            return timeline.getLast().msg;
        } finally {
            r.unlock();
        }
    }

    public @Nullable MessageDataHeaderDto getLastHeaderOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getData().getHeader();
    }

    public @Nullable Long getLastOffsetOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getMeta().getOffset();
    }

    public @Nullable MessageDataDto getLastDataOrNull() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return null;
        return last.getData();
    }

    public String getPayloadClazz() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return "[null]";
        return lastHeader.getPayloadClazzOrThrow();
    }

    public boolean getImmutable() {
        MessageDataHeaderDto lastHeader = getLastHeaderOrNull();
        if (lastHeader == null) return false;
        return lastHeader.getInheritWrite() == false && lastHeader.getAllowWrite().isEmpty();
    }

    public boolean hasPayload() {
        MessageDataMetaDto last = getLastOrNull();
        if (last == null) return false;
        return last.getData().hasPayload();
    }

    public Iterable<MessageMetaDto> getHistory() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            return this.timeline.stream()
                    .map(a -> a.msg.getMeta())
                    .collect(Collectors.toList());
        } finally {
            r.unlock();
        }
    }

    private @Nullable LinkedList<DataGraphNode> computeCurrentLeaves() {
        Lock r = this.lock.readLock();
        r.lock();
        try {
            if (this.leaves.isEmpty()) return null;

            LinkedList<DataGraphNode> ret = new LinkedList<>();
            for (DataGraphNode node : this.leaves) {
                ret.add(node);
            }
            return ret;
        } finally {
            r.unlock();
        }
    }

    public MessageDataHeaderDto getMergedHeader() {
        AteDelegate d = AteDelegate.get();

        LinkedList<DataGraphNode> leaves = computeCurrentLeaves();
        if (leaves == null || leaves.isEmpty()) throw new RuntimeException("Unable to getData the merged header(#1).");

        // If there is only one item then we are done
        if (leaves.size() == 1) {
            return leaves.get(0).msg.getData().getHeader();
        }

        // Build a merge set of the headers for this
        ArrayList<MergePair<MessageDataHeaderDto>> mergeSet = new ArrayList<>();
        leaves.stream().map(n -> new MergePair<>(
                (n.parentNode != null ? n.parentNode.msg.getData().getHeader() : null),
                n.msg.getData().getHeader()))
            .forEach(a -> mergeSet.add(a));

        // Return the result of the merge
        MessageDataHeaderDto ret = d.merger.merge(mergeSet);
        if (ret == null) throw new RuntimeException("Unable to getData the merged header(#2).");
        return ret;
    }

    private static @Nullable BaseDao reconcileMergedData(@Nullable BaseDao _ret, LinkedList<DataGraphNode> leaves) {
        AteDelegate d = AteDelegate.get();
        BaseDao ret = _ret;
        if (ret == null) return null;
        IPartitionKey partitionKey = d.io.partitionResolver().resolve(ret);

        // Reconcile the parent version pointers
        if (leaves.size() == 1) {
            ret.previousVersion = leaves.getLast().version;
        } else {
            ret.previousVersion = null;
            ret.version = UUID.randomUUID();
            ret.mergesVersions = leaves.stream().map(n -> n.version).collect(Collectors.toSet());

            // If a mergeThreeWay was performed and we have writability then we should save it down to reduce future merges and
            // so that log compaction doesnt lose data (Kafka compacting)
            if (leaves.size() > 1) {
                EffectivePermissions perms = d.authorization.perms(partitionKey, ret.getId(), ret.getParentId(), false);
                if (perms.canWrite(d.currentRights)) {
                    d.io.mergeAsyncWithoutValidation(ret);
                }
            }
        }

        return ret;
    }

    @SuppressWarnings("return.type.incompatible")
    public @Nullable BaseDao getMergedData() {
        AteDelegate d = AteDelegate.get();
        BaseDao ret;

        LinkedList<DataGraphNode> leaves = computeCurrentLeaves();
        if (leaves == null || leaves.isEmpty()) return null;

        // If there is only one item then we are done
        if (leaves.size() == 1) {
            return d.dataSerializer.fromDataMessage(this.partitionKey, leaves.get(0).msg, true);
        }

        // Build a merge set of the headers for this
        Map<DataGraphNode, BaseDao> deserializeCache = new HashMap<>();
        List<MergePair<BaseDao>> mergeSet = leaves
                .stream().map(n -> new MergePair<>(
                        n.parentNode != null ? deserializeCache.computeIfAbsent(n.parentNode, v -> d.dataSerializer.fromDataMessage(this.partitionKey, v.msg, true)) : null,
                        deserializeCache.computeIfAbsent(n, v -> d.dataSerializer.fromDataMessage(this.partitionKey, n.msg, true))))
                .collect(Collectors.toList());
        MergePair<BaseDao> last = mergeSet.get(mergeSet.size()-1);

        // Merge the actual merge of the data object
        ret = d.merger.merge(mergeSet);
        return reconcileMergedData(ret, leaves);
    }
}