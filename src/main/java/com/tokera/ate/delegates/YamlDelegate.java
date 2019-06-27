package com.tokera.ate.delegates;

import com.esotericsoftware.yamlbeans.Version;
import com.esotericsoftware.yamlbeans.YamlConfig;
import com.esotericsoftware.yamlbeans.YamlException;
import com.esotericsoftware.yamlbeans.YamlReader;
import com.esotericsoftware.yamlbeans.YamlWriter;
import com.tokera.ate.annotations.YamlTag;
import com.tokera.ate.annotations.YamlTags;
import com.tokera.ate.dao.PUUID;
import com.tokera.ate.extensions.YamlTagDiscoveryExtension;
import com.tokera.ate.io.api.IPartitionKey;
import com.tokera.ate.providers.*;
import org.apache.commons.lang.math.LongRange;

import java.beans.IntrospectionException;
import java.beans.Introspector;
import java.beans.PropertyDescriptor;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.Reader;
import java.io.StringWriter;
import java.io.Writer;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.*;
import javax.annotation.PostConstruct;
import javax.enterprise.context.ApplicationScoped;
import javax.inject.Inject;
import javax.ws.rs.WebApplicationException;

/**
 * Delegate used for the serialization and deserialization of java objects into YAML text format
 */
@ApplicationScoped
public class YamlDelegate {

    @SuppressWarnings("initialization.fields.uninitialized")
    @Inject
    private YamlTagDiscoveryExtension discovery;

    @SuppressWarnings("initialization.fields.uninitialized")
    private YamlConfig m_config;
    @SuppressWarnings("initialization.fields.uninitialized")
    private YamlConfig m_config_no_marker;

    public void init(YamlTagDiscoveryExtension discovery) {
        this.discovery = discovery;
        init();
    }
    
    @PostConstruct
    public void init() {
        m_config = new YamlConfig();
        
        YamlDelegate.initConfig(m_config, discovery);
        
        m_config_no_marker = new YamlConfig();
        YamlDelegate.initConfig(m_config_no_marker, discovery);
        m_config_no_marker.writeConfig.setExplicitFirstDocument(false);
        m_config_no_marker.writeConfig.setExplicitEndDocument(false);
    }

    @SuppressWarnings({"unchecked.method.invocation", "unchecked.conversion", "known.nonnull"})
    public static void initConfig(YamlConfig cfg, YamlTagDiscoveryExtension discovery)
    {
        for (Class<?> clazz : discovery.getYamlTagClasses()) {
            
            List<YamlTag> tags = new ArrayList<>();
            
            YamlTags tagsAtt = clazz.getAnnotation(YamlTags.class);
            if (tagsAtt != null) tags.addAll(Arrays.asList(tagsAtt.value()));
            else tags.addAll(Arrays.asList(clazz.getAnnotationsByType(YamlTag.class)));
                    
            for (YamlTag tag : tags) {
                cfg.setClassTag(tag.value(), clazz);
            }
            
            if (clazz.isEnum()) {
                cfg.setScalarSerializer(clazz, new EnumSerializer(clazz));
            }

            try {
                for (PropertyDescriptor prop : Introspector.getBeanInfo(clazz).getPropertyDescriptors()) {
                    if (prop.getReadMethod() == null) continue;
                    if (prop.getWriteMethod() == null) continue;
                    Type propType = prop.getReadMethod().getGenericReturnType();

                    try {
                        Class<?> propClazz = prop.getPropertyType();
                        if (propClazz == null) continue;

                        if (Collection.class.isAssignableFrom(propClazz) || Map.class.isAssignableFrom(propClazz)) {
                            ParameterizedType aType = (ParameterizedType)propType;
                            if (aType == null) continue;
                            Type[] fieldArgTypes = aType.getActualTypeArguments();
                            if (fieldArgTypes.length == 1) {
                                if (fieldArgTypes[0] instanceof Class) {
                                    Class<?> elementType = (Class) fieldArgTypes[0];
                                    cfg.setPropertyElementType(clazz, prop.getName(), elementType);
                                }
                            }
                        }
                    } catch (Throwable e) {
                        AteDelegate.get().genericLogger.warn(e);
                    }
                }
            } catch (IntrospectionException e) {
                throw new RuntimeException(e);
            }
        }
        
        cfg.setClassTag("dictionary", java.util.Dictionary.class);
        cfg.setClassTag("hashmap", java.util.HashMap.class);
        cfg.setClassTag("hashset", java.util.HashSet.class);
        cfg.setClassTag("hashtable", java.util.Hashtable.class);
        cfg.setClassTag("arraylist", java.util.ArrayList.class);
        cfg.setClassTag("linkedlist", java.util.LinkedList.class);
        cfg.setClassTag("linkedhashmap", java.util.LinkedHashMap.class);
        cfg.setClassTag("linkedhashset", java.util.LinkedHashSet.class);
        cfg.setClassTag("stack", java.util.Stack.class);
        cfg.setClassTag("treemap", java.util.TreeMap.class);
        cfg.setClassTag("treeset", java.util.TreeSet.class);
        cfg.setClassTag("vector", java.util.Vector.class);
        cfg.setClassTag("weakhashmap", java.util.WeakHashMap.class);

        //cfg.setClassTag("bool", Boolean.class);
        //cfg.setClassTag("long", Long.class);
        //cfg.setClassTag("int", Integer.class);
        cfg.setClassTag("timestamp", java.util.Date.class);
        cfg.setClassTag("uuid", java.util.UUID.class);
        cfg.setClassTag("longrange", LongRange.class);
        cfg.setClassTag("key", IPartitionKey.class);
        cfg.setClassTag("puuid", PUUID.class);
        cfg.setClassTag("bigdecimal", java.math.BigDecimal.class);
        
        //cfg.setScalarSerializer(Boolean.class, new BooleanSerializer());
        //cfg.setScalarSerializer(Long.class, new LongSerializer());
        //cfg.setScalarSerializer(Integer.class, new IntegerSerializer());
        cfg.setScalarSerializer(java.util.Date.class, new DateSerializer());
        cfg.setScalarSerializer(java.util.UUID.class, new UuidSerializer());
        cfg.setScalarSerializer(PUUID.class, new PuuidSerializer());
        cfg.setScalarSerializer(LongRange.class, new RangeLongSerializer());
        cfg.setScalarSerializer(IPartitionKey.class, new PartitionKeySerializer());
        cfg.setScalarSerializer(java.math.BigDecimal.class, new BigDecimalSerializer());

        cfg.setAllowDuplicates(true);
        
        cfg.readConfig.setIgnoreUnknownProperties(true);
        
        cfg.writeConfig.setAutoAnchor(false);
        cfg.writeConfig.setVersion(new Version(1, 2));
        cfg.writeConfig.setWriteDefaultValues(false);
        cfg.writeConfig.setIndentSize(2);
        cfg.writeConfig.setEscapeUnicode(false);
        cfg.writeConfig.setExplicitFirstDocument(false);
        cfg.writeConfig.setExplicitEndDocument(true);
        cfg.writeConfig.setWrapColumn(Integer.MAX_VALUE);
        cfg.writeConfig.setWriteClassname(YamlConfig.WriteClassName.AUTO);
    }
    
    public YamlWriter getYamlWriter(Writer writer, boolean addMarker) {
        if (addMarker) {
            return new YamlWriter(writer, m_config);
        } else {
            return new YamlWriter(writer, m_config_no_marker);
        }
    }
    
    public YamlReader getYamlReader(String yaml) {
        return new YamlReader(yaml, m_config);
    }
    
    public YamlReader getYamlReader(Reader reader) {
        return new YamlReader(reader, m_config);
    }
    
    public String serializeObj(Object obj)
    {
        return serializeObj(obj, true);
    }
    
    public String serializeObj(Object obj, boolean addMarker)
    {
        try {            
            StringWriter sw = new StringWriter();
            YamlWriter writer = getYamlWriter(sw, addMarker);
            writer.write(obj);
            writer.close();
            String body = sw.getBuffer().toString();
            return body;
            //return StringTools.unescapeLines(body);
        } catch (YamlException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public Object deserializeObj(String yaml)
    {
        try {
            YamlReader reader = getYamlReader(yaml);
            return reader.read();
        } catch (YamlException ex) {
            throw new RuntimeException(ex);
        }
    }
    
    public byte[] serializeObjBytes(Object obj)
    {
        try {
            ByteArrayOutputStream stream = new ByteArrayOutputStream();
            OutputStreamWriter streamWriter = new OutputStreamWriter(stream);
            YamlWriter writer = getYamlWriter(streamWriter, true);
            writer.write(obj);
            writer.close();
            return stream.toByteArray();
        } catch (YamlException ex) {
            throw new WebApplicationException(ex);
        }
    }
    
    public Object deserializeObjBytes(byte[] yaml)
    {
        try {
            ByteArrayInputStream stream = new ByteArrayInputStream(yaml);
            InputStreamReader streamReader = new InputStreamReader(stream);
            YamlReader reader = getYamlReader(streamReader);
            return reader.read();
        } catch (YamlException ex) {
            throw new WebApplicationException(ex);
        }
    }
}